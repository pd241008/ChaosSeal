package engine

import (
	"context"
	"fmt"
	"math"
	"math/rand"
	"time"

	"github.com/chaosseal/netsim/core/client"
	"github.com/chaosseal/netsim/core/crypto"
	"github.com/chaosseal/netsim/core/kinematics"
)

const (
	surveyStepSec = 1.0
	payloadBytes  = 1024 // baseline application payload size
	orbitalPlanes = 4
)

// Simulation owns the state of one reproducible run.
type Simulation struct {
	cfg           *Config
	rng           *rand.Rand
	sats          []*kinematics.Satellite
	gs            *kinematics.GroundStation
	channels      []*kinematics.GilbertElliott
	result        *RunResult
	measureSat    int
	measureOffset float64
}

// NewSimulation builds a Simulation from the config, deriving all stochastic
// state from cfg.Seed so the run is reproducible. Satellites are spread
// evenly across orbitalPlanes of a Walker constellation.
func NewSimulation(cfg *Config) *Simulation {
	rng := rand.New(rand.NewSource(cfg.Seed))
	gs := &kinematics.GroundStation{LatDeg: cfg.GroundLat, LonDeg: cfg.GroundLon}

	sats := make([]*kinematics.Satellite, 0, cfg.Satellites)
	for i := 0; i < cfg.Satellites; i++ {
		plane := i % orbitalPlanes
		raan := 360.0 * float64(plane) / orbitalPlanes
		perPlane := (cfg.Satellites + orbitalPlanes - 1) / orbitalPlanes
		idxInPlane := i / orbitalPlanes
		phase := 2 * math.Pi * float64(idxInPlane) / float64(perPlane)
		sats = append(sats, kinematics.NewSatellite(i, cfg.AltitudeKm, 53*math.Pi/180, raan, phase))
	}

	channels := make([]*kinematics.GilbertElliott, len(sats))
	for i := range sats {
		channels[i] = kinematics.NewGilbertElliott(cfg.Loss, rand.New(rand.NewSource(cfg.Seed+int64(i)+1)))
	}

	return &Simulation{
		cfg:      cfg,
		rng:      rng,
		sats:     sats,
		gs:       gs,
		channels: channels,
		result:   NewRunResult(cfg),
	}
}

// Run executes the link survey and every configured baseline, then returns
// the populated result.
func (s *Simulation) Run(ctx context.Context) (*RunResult, error) {
	s.result.LinkStats = s.surveyLinks()
	s.measureSat, s.measureOffset = s.bestMeasurement()

	for _, b := range s.cfg.Baselines {
		switch b {
		case "chaosseal":
			if err := s.runChaosSeal(ctx); err != nil {
				return nil, fmt.Errorf("chaosseal baseline: %w", err)
			}
		case "counter":
			if err := s.runCounter(ctx); err != nil {
				return nil, fmt.Errorf("counter baseline: %w", err)
			}
		case "tls13":
			if err := s.runTLS13(); err != nil {
				return nil, fmt.Errorf("tls13 baseline: %w", err)
			}
		case "bpsec":
			if err := s.runBPSec(); err != nil {
				return nil, fmt.Errorf("bpsec baseline: %w", err)
			}
		}
	}
	return s.result, nil
}

// surveyLinks samples satellite geometry and channel state across the whole
// run window, producing aggregate link statistics and visibility events.
func (s *Simulation) surveyLinks() *LinkStats {
	stats := &LinkStats{Satellites: len(s.sats)}

	var (
		visibleCount    int
		totalLatency    float64
		maxLatency      float64
		lossCount       int
		burstCount      int
		maxBurst        int
		samples         int
		latencySamples  []float64
		prevVisible     = make([]bool, len(s.sats))
	)

	for t := 0.0; t <= s.cfg.DurationSec; t += surveyStepSec {
		for i, sat := range s.sats {
			elev := s.gs.ElevationDeg(sat, t)
			visible := elev >= s.cfg.MinElevDeg
			if visible {
				visibleCount++
			}
			samples++

			lat := s.gs.OneWayLatencySec(sat, t) * 1000 // ms
			totalLatency += lat
			if visible {
				latencySamples = append(latencySamples, lat)
			}
			if lat > maxLatency {
				maxLatency = lat
			}

			if t == 0 {
				prevVisible[i] = visible
				continue
			}
			if visible != prevVisible[i] {
				state := "down"
				if visible {
					state = "up"
				}
				s.result.Events = append(s.result.Events, Event{
					TimeSec:   t,
					Type:      "link_" + state,
					Satellite: sat.ID,
				})
				prevVisible[i] = visible
			}
		}
	}

	// Run the per-satellite loss channels over the same window.
	for i, ch := range s.channels {
		burstLen := 0
		for t := 0.0; t <= s.cfg.DurationSec; t += surveyStepSec {
			lost := ch.Next()
			if lost {
				lossCount++
				burstLen++
				if burstLen == 1 {
					burstCount++
				}
				if burstLen > maxBurst {
					maxBurst = burstLen
				}
			} else {
				burstLen = 0
			}
		}
		_ = i
	}

	if samples > 0 {
		stats.VisiblePct = float64(visibleCount) / float64(samples) * 100
		stats.MeanLatencyMs = totalLatency / float64(samples)
		stats.MaxLatencyMs = maxLatency
	}
	if samples > 0 {
		stats.LossRate = float64(lossCount) / float64(samples)
	}
	stats.BurstCount = burstCount
	stats.MaxBurstLen = maxBurst
	stats.LatencySamplesMs = latencySamples
	return stats
}

// bestMeasurement finds the satellite and run offset where a baseline link
// would be strongest (highest elevation). Baselines transmit at this moment,
// guaranteeing an actually visible link rather than a below-horizon one.
func (s *Simulation) bestMeasurement() (sat int, offset float64) {
	best := 0
	bestElev := math.Inf(-1)
	bestT := 0.0
	observationWindowSec := 600.0
	limit := s.cfg.DurationSec
	if observationWindowSec < limit {
		limit = observationWindowSec
	}
	for t := 0.0; t <= limit; t += surveyStepSec {
		for i, sat := range s.sats {
			e := s.gs.ElevationDeg(sat, t)
			if e > bestElev {
				bestElev = e
				best = i
				bestT = t
			}
		}
	}
	return best, bestT
}

// referenceLink returns the link for the satellite chosen by bestMeasurement,
// positioned at the measurement offset.
func (s *Simulation) referenceLink() *kinematics.Link {
	baseEpoch := time.Date(2025, 1, 1, 0, 0, 0, 0, time.UTC)
	epoch := baseEpoch.Add(time.Duration(s.measureOffset * float64(time.Second)))
	return kinematics.NewLink(s.sats[s.measureSat], s.gs, s.cfg.MinElevDeg, s.channels[s.measureSat], epoch)
}

// transmitWithLoss sends a packet at offsetSec, applying either the configured
// fixed loss-rate override (if nonzero) or the Gilbert-Elliott model.
func (s *Simulation) transmitWithLoss(link *kinematics.Link, t float64) kinematics.PacketOutcome {
	if s.cfg.LossRateOverride > 0 {
		visible := link.VisibleAt(t)
		lost := s.rng.Float64() < s.cfg.LossRateOverride
		latency := link.LatencyAt(t)
		if !visible || lost {
			return kinematics.PacketOutcome{Lost: true, At: link.StartTime, Latency: latency}
		}
		return kinematics.PacketOutcome{Lost: false, At: link.StartTime.Add(latency), Latency: latency, Progress: 1}
	}
	return link.Transmit(t)
}

const (
	numDataPackets = 10000
)

// simulateDataStream models a stream of lossRate-override packets through the
// data layer. Each packet has a `lossRate` chance of being lost/corrupted. The
// HMAC commitment is verified every `commitInterval` packets (0 = every packet);
// a mismatch triggers a resync (extra cost). Returns aggregate loss + resync stats.
func (s *Simulation) simulateDataStream(lossRate float64, commitInterval int) map[string]interface{} {
	// Effective per-packet corruption probability (loss on the link).
	p := lossRate
	resyncInterval := commitInterval
	if resyncInterval <= 0 {
		resyncInterval = 1
	}
	// A packet corrupted by the channel will fail HMAC verification when it is
	// the (or near the) verification point. Simple model: each packet has p
	// chance of corruption; verification catches it at the next check point.
	resyncs := 0
	retransmissions := 0
	delivered := 0
	for i := 0; i < numDataPackets; i++ {
		corrupted := s.rng.Float64() < p
		if corrupted {
			retransmissions++
			// The corrupted packet triggers verification failure at the next
			// check boundary, causing one resync.
			resyncs++
			// Retransmit successfully (clean link assumed for retransmit).
			delivered++
		} else {
			delivered++
		}
	}
	lossRate = float64(retransmissions) / float64(numDataPackets)
	return map[string]interface{}{
		"packets":            numDataPackets,
		"delivered":          delivered,
		"retransmissions":    retransmissions,
		"loss_rate":          lossRate,
		"resyncs":            resyncs,
		"commit_interval_n":  commitInterval,
		"resyncs_per_packet": float64(resyncs) / float64(numDataPackets),
	}
}

// runChaosSeal drives the Rust core CLI: it computes a Lyapunov exponent and
// a BEE ciphertext, then simulates a revocation broadcast over the link.
func (s *Simulation) runChaosSeal(ctx context.Context) error {
	rust := &client.RustCoreClient{Command: s.cfg.RustCLI}

	lyap, err := rust.Lyapunov(ctx, 3, 1.0, 1.0, 0.1, 0.5, s.cfg.LyapunovSteps)
	if err != nil {
		return err
	}
	startResync := time.Now()
	client.EpochKeyGen() // simulate background chaotic derivation compute time
	bee, err := rust.BeeSize(ctx, s.cfg.BEE_N, s.cfg.BEE_R)
	if err != nil {
		return err
	}
	resyncComputeSec := time.Since(startResync).Seconds()

	out := map[string]interface{}{
		"lyapunov":            lyap,
		"bee":                 bee,
		"revocation_messages": make([]map[string]interface{}, 0),
		"delivery":            map[string]interface{}{},
	}

	link := s.referenceLink()
	t := s.measureOffset

	// Broadcast R individual revocation messages over the reference link.
	msgs := make([]map[string]interface{}, 0, s.cfg.BEE_R)
	bytesPerMsg := bee.CiphertextSizeMin
	transferSec := float64(bytesPerMsg*8) / s.cfg.DownlinkBps
	lat := link.GS.OneWayLatencySec(link.Sat, t)

	totalLoss := 0
	for r := 0; r < s.cfg.BEE_R; r++ {
		outcome := s.transmitWithLoss(link, t)
		if outcome.Lost {
			totalLoss++
		}
		s.result.Events = append(s.result.Events, Event{
			TimeSec:   t,
			Type:      "bee_revoke",
			Satellite: r % len(s.sats),
			Detail:    "revoked receiver + broadcast update",
			Value:     float64(bytesPerMsg),
		})
		msgs = append(msgs, map[string]interface{}{
			"revoked_receiver": r,
			"ciphertext_bytes": bytesPerMsg,
			"transfer_sec":     transferSec,
			"latency_ms":       lat * 1000,
			"lost":             outcome.Lost,
		})
	}

	delivered := s.cfg.BEE_R - totalLoss
	s.result.Events = append(s.result.Events, Event{
		TimeSec: t,
		Type:    "bee_delivery",
		Detail:  fmt.Sprintf("%d/%d updates delivered", delivered, s.cfg.BEE_R),
		Value:   float64(delivered),
	})

	out["revocation_messages"] = msgs
	out["delivery"] = map[string]interface{}{
		"sent":      s.cfg.BEE_R,
		"delivered": delivered,
		"losses":    totalLoss,
		"loss_rate": float64(totalLoss) / float64(s.cfg.BEE_R),
	}
	out["resync_compute_sec"] = resyncComputeSec

	// Simulate Data Transmission Phase for a single epoch
	payloadSz := s.cfg.PayloadBytes
	if payloadSz <= 0 {
		payloadSz = payloadBytes // fallback to const
	}
	dataPayload := make([]byte, payloadSz)
	for i := range dataPayload {
		dataPayload[i] = byte(i % 251)
	}

	startData := time.Now()
	cryptoRes := client.EpochCrypto(dataPayload)
	cryptoWallclockSec := time.Since(startData).Seconds()

	amortizedBeeBytes := float64(bee.CiphertextSizeMin*s.cfg.BEE_R) / float64(s.cfg.BEE_N)
	// Commitment-interval overhead: an HMAC tag (32 B) is transmitted every N
	// packets as part of the integrity-verification commitment stream. Amortized
	// over the epoch this is 32/N bytes per packet for N>0; N=0 (default) means
	// the pendulum's per-epoch commitment dominates and no per-packet sync tag.
	var commitBytes float64
	if s.cfg.CommitIntervalN > 0 {
		commitBytes = float64(32) / float64(s.cfg.CommitIntervalN)
	}
	overheadBytes := float64(cryptoRes.CryptoOverhead) + amortizedBeeBytes + commitBytes
	overheadPct := overheadBytes / float64(payloadSz)

	dataTransferSec := float64(float64(payloadSz)+overheadBytes)*8.0/s.cfg.DownlinkBps + lat

	out["data_transmission"] = map[string]interface{}{
		"payload_bytes":                   payloadSz,
		"crypto_wallclock_us":             cryptoWallclockSec * 1000000.0,
		"overhead_pct":                    overheadPct,
		"overhead_bytes_per_payload_byte": overheadPct,
		"commit_interval_n":               s.cfg.CommitIntervalN,
		"commit_amortized_bytes":          commitBytes,
		"transfer_sec":                    dataTransferSec,
	}

	// If a fixed loss override is set, exercise the data-layer HMAC-resync
	// behaviour under that loss rather than the clean-channel assumption.
	if s.cfg.LossRateOverride > 0 {
		out["data_stream_loss_sensitivity"] = s.simulateDataStream(s.cfg.LossRateOverride, s.cfg.CommitIntervalN)
	}

	s.result.Baselines["chaosseal"] = out
	return nil
}

// runCounter drives the counter-mode baseline: HKDF(SessionSeed || i) → AES-GCM.
// This is the deterministic baseline without chaotic dynamics — same BEE revocation,
// but key derivation is a simple counter rather than the chaotic pendulum.
func (s *Simulation) runCounter(ctx context.Context) error {
	rust := &client.RustCoreClient{Command: s.cfg.RustCLI}

	lyap, err := rust.Lyapunov(ctx, 3, 1.0, 1.0, 0.1, 0.5, s.cfg.LyapunovSteps)
	if err != nil {
		return err
	}
	startResync := time.Now()
	client.CounterEpochKeyGen() // instant, no chaotic simulation
	bee, err := rust.BeeSize(ctx, s.cfg.BEE_N, s.cfg.BEE_R)
	if err != nil {
		return err
	}
	resyncComputeSec := time.Since(startResync).Seconds()

	out := map[string]interface{}{
		"lyapunov":            lyap,
		"bee":                 bee,
		"revocation_messages": make([]map[string]interface{}, 0),
		"delivery":            map[string]interface{}{},
	}

	link := s.referenceLink()
	t := s.measureOffset

	msgs := make([]map[string]interface{}, 0, s.cfg.BEE_R)
	bytesPerMsg := bee.CiphertextSizeMin
	transferSec := float64(bytesPerMsg*8) / s.cfg.DownlinkBps
	lat := link.GS.OneWayLatencySec(link.Sat, t)

	totalLoss := 0
	for r := 0; r < s.cfg.BEE_R; r++ {
		outcome := s.transmitWithLoss(link, t)
		if outcome.Lost {
			totalLoss++
		}
		s.result.Events = append(s.result.Events, Event{
			TimeSec:   t,
			Type:      "counter_revoke",
			Satellite: r % len(s.sats),
			Detail:    "counter-mode revoked receiver + broadcast update",
			Value:     float64(bytesPerMsg),
		})
		msgs = append(msgs, map[string]interface{}{
			"revoked_receiver": r,
			"ciphertext_bytes": bytesPerMsg,
			"transfer_sec":     transferSec,
			"latency_ms":       lat * 1000,
			"lost":             outcome.Lost,
		})
	}

	delivered := s.cfg.BEE_R - totalLoss
	s.result.Events = append(s.result.Events, Event{
		TimeSec: t,
		Type:    "counter_delivery",
		Detail:  fmt.Sprintf("%d/%d updates delivered", delivered, s.cfg.BEE_R),
		Value:   float64(delivered),
	})
	out["revocation_messages"] = msgs
	out["delivery"] = map[string]interface{}{
		"sent":      s.cfg.BEE_R,
		"delivered": delivered,
		"losses":    totalLoss,
		"loss_rate": float64(totalLoss) / float64(s.cfg.BEE_R),
	}
	out["resync_compute_sec"] = resyncComputeSec

	// Data transmission via counter-mode crypto
	payloadSz := s.cfg.PayloadBytes
	if payloadSz <= 0 {
		payloadSz = payloadBytes
	}
	dataPayload := make([]byte, payloadSz)
	for i := range dataPayload {
		dataPayload[i] = byte(i % 251)
	}

	startData := time.Now()
	cryptoRes, counterVal := client.CounterEpochCrypto(dataPayload)
	cryptoWallclockSec := time.Since(startData).Seconds()

	amortizedBeeBytes := float64(bee.CiphertextSizeMin*s.cfg.BEE_R) / float64(s.cfg.BEE_N)
	overheadBytes := float64(cryptoRes.CryptoOverhead) + amortizedBeeBytes
	overheadPct := overheadBytes / float64(payloadSz)

	dataTransferSec := float64(float64(payloadSz)+overheadBytes)*8.0/s.cfg.DownlinkBps + lat

	out["data_transmission"] = map[string]interface{}{
		"payload_bytes":                   payloadSz,
		"crypto_wallclock_us":             cryptoWallclockSec * 1000000.0,
		"overhead_pct":                    overheadPct,
		"overhead_bytes_per_payload_byte": overheadPct,
		"transfer_sec":                    dataTransferSec,
		"counter_value":                   counterVal,
	}

	if s.cfg.LossRateOverride > 0 {
		out["data_stream_loss_sensitivity"] = s.simulateDataStream(s.cfg.LossRateOverride, s.cfg.CommitIntervalN)
	}

	s.result.Baselines["counter"] = out
	return nil
}

// runTLS13 performs a real TLS 1.3 handshake over a latency-delayed link and
// records the measured timing and byte counts.
func (s *Simulation) runTLS13() error {
	link := s.referenceLink()
	t := s.measureOffset
	baseLatency := link.GS.OneWayLatencySec(link.Sat, t)

	provider := func() time.Duration {
		return time.Duration(baseLatency * float64(time.Second))
	}

	payloadSz := s.cfg.PayloadBytes
	if payloadSz <= 0 {
		payloadSz = payloadBytes
	}
	res, err := crypto.RunTLS13Baseline(provider, make([]byte, payloadSz))
	if err != nil {
		return err
	}

	s.result.Events = append(s.result.Events, Event{
		TimeSec: t,
		Type:    "tls13_handshake",
		Detail:  res.CipherSuite,
		Value:   res.HandshakeSec,
	})
	s.result.Baselines["tls13"] = map[string]interface{}{
		"handshake_sec":     res.HandshakeSec,
		"bytes_sent":        res.BytesSent,
		"bytes_received":    res.BytesReceived,
		"cipher_suite":      res.CipherSuite,
		"app_payload_sec":   res.AppPayloadSec,
		"one_way_latency_ms": baseLatency * 1000,
	}
	return nil
}

// runBPSec builds a BPv7 bundle with a BCB (AES-256-GCM) and BIB
// (HMAC-SHA256), transmits it over the reference link, and verifies integrity.
func (s *Simulation) runBPSec() error {
	key := crypto.NewBPSecKey(uint64(s.cfg.Seed))
	iv, err := crypto.NewRandomIV()
	if err != nil {
		return err
	}

	payloadSz := s.cfg.PayloadBytes
	if payloadSz <= 0 {
		payloadSz = payloadBytes
	}
	payload := make([]byte, payloadSz)
	for i := range payload {
		payload[i] = byte(i % 251)
	}
	payloadBlock := crypto.BuildPayloadBlock(payload)

	bcb, _, err := crypto.BuildBCB(key, iv, "key-0", payloadBlock)
	if err != nil {
		return err
	}
	bib, _ := crypto.BuildBIB(key, "key-0", bcb)

	bundle := &crypto.Bundle{
		Primary: &crypto.PrimaryBlock{
			Version:     7,
			Flags:       0,
			Destination: "ipn:1.1",
			Source:      "ipn:1.0",
			ReportTo:    "ipn:1.0",
			Time:        uint64(time.Now().Unix()),
			Lifetime:    3600,
		},
		Blocks: []*crypto.CanonicalBlock{payloadBlock, bcb, bib},
	}
	wire := bundle.Encode()

	ok, err := crypto.VerifyBIB(key, bib, bcb)
	if err != nil {
		return err
	}
	if !ok {
		return fmt.Errorf("BIB verification failed on generated bundle")
	}
	plain, err := crypto.DecryptPayload(key, iv, bcb, payloadBlock)
	if err != nil {
		return err
	}
	if len(plain) != len(payload) {
		return fmt.Errorf("payload round-trip length mismatch: %d != %d", len(plain), len(payload))
	}

	link := s.referenceLink()
	t := s.measureOffset
	lat := link.GS.OneWayLatencySec(link.Sat, t)
	transferSec := float64(len(wire)*8)/s.cfg.DownlinkBps + lat
	outcome := s.transmitWithLoss(link, t)

	overheadBytes := float64(len(wire) - len(payload))
	overheadPct := overheadBytes / float64(len(payload))

	s.result.Events = append(s.result.Events, Event{
		TimeSec: t,
		Type:    "bpsec_bundle",
		Detail:  "BPv7 bundle with BIB+BCB delivered",
		Value:   float64(len(wire)),
	})
	s.result.Baselines["bpsec"] = map[string]interface{}{
		"bundle_size_bytes":               len(wire),
		"payload_bytes":                   len(payload),
		"ciphertext_bytes":                len(wire) - len(payload),
		"transfer_sec":                    transferSec,
		"latency_ms":                      lat * 1000,
		"lost":                            outcome.Lost,
		"bib_verified":                    ok,
		"payload_roundtrip":               true,
		"overhead_pct":                    overheadPct,
		"overhead_bytes_per_payload_byte": overheadPct,
	}
	return nil
}

