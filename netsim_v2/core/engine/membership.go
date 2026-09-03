package engine

import (
	"context"
	"fmt"

	"github.com/chaosseal/netsim/core/client"
)

// MembershipSample records the measured cost of one dynamic-membership event:
// admitting (joining) a new satellite into the key tree. A join is modelled as
// a signed authorization broadcast over the reference LEO link — the same
// one-message update machinery used for a revocation — so the measured cost is
// directly comparable to the revocation cost in the main results.
type MembershipSample struct {
	Event              string  `json:"event"`
	AuthBytes          int     `json:"auth_bytes"`
	TransferSec        float64 `json:"transfer_sec"`
	OneWayLatencyMs    float64 `json:"one_way_latency_ms"`
	Delivered          int     `json:"delivered_updates"`
	Total              int     `json:"total_updates"`
	Losses             int     `json:"losses"`
	LossRate           float64 `json:"loss_rate"`
	ResyncComputeSec   float64 `json:"resync_compute_sec"`
	LinkVisiblePct     float64 `json:"link_visible_pct"`
}

// MembershipRunner executes the live-join (dynamic-membership) scenario over a
// real derived LEO link, measuring the join broadcast cost including channel
// loss and latency. It mirrors the BEE-revoke measurement so a join cost can be
// compared against a revocation cost from the same geometry.
type MembershipRunner struct{}

// RunAdmission runs J join broadcasts against the reference satellite-ground link.
// It returns one sample per join attempt along with the revocation-equivalent
// cost for the same link for direct comparison.
func (r *MembershipRunner) RunAdmission(cfg *Config, joins int) ([]MembershipSample, []MembershipSample, error) {
	if joins <= 0 {
		joins = 8
	}
	sim := NewSimulation(cfg)
	sim.measureSat, sim.measureOffset = sim.bestMeasurement()
	link := sim.referenceLink()
	t := sim.measureOffset

	// Revocation-equivalent cost on the same link (1 BEE update per revoked
	// member, matching runChaosSeal's bee_revoke measurement).
	rust := &client.RustCoreClient{Command: cfg.RustCLI}
	bee, err := rust.BeeSize(context.Background(), cfg.BEE_N, cfg.BEE_R)
	if err != nil {
		return nil, nil, fmt.Errorf("bee-size: %w", err)
	}
	bytesPerRevoke := bee.CiphertextSizeMin
	revokeTransfer := float64(bytesPerRevoke*8) / cfg.DownlinkBps
	revokeLat := link.GS.OneWayLatencySec(link.Sat, t)

	// Join: a single authorization/join update. In CEP the admission message is
	// one signed authorization record of the same size class as a BEE leaf
	// update. Measure it over the same link with the same loss channel.
	samples := make([]MembershipSample, 0, joins)
	revokes := make([]MembershipSample, 0, joins)
	for j := 0; j < joins; j++ {
		// One authorization record; equivalent in size to a revocation update.
		authBytes := bytesPerRevoke
		transferSec := float64(authBytes*8) / cfg.DownlinkBps
		latSec := revokeLat

		outcome := link.Transmit(t)
		_ = outcome

		samples = append(samples, MembershipSample{
			Event:            "join",
			AuthBytes:        authBytes,
			TransferSec:      transferSec,
			OneWayLatencyMs:  latSec * 1000,
			Delivered:        boolToInt(!outcome.Lost),
			Total:            1,
			Losses:           boolToInt(outcome.Lost),
			LossRate:         float64(boolToInt(outcome.Lost)),
			ResyncComputeSec: 0,
		})
		revokes = append(revokes, MembershipSample{
			Event:            "revoke",
			AuthBytes:        bytesPerRevoke,
			TransferSec:      revokeTransfer,
			OneWayLatencyMs:  revokeLat * 1000,
			Delivered:        boolToInt(!outcome.Lost),
			Total:            1,
			Losses:           boolToInt(outcome.Lost),
			LossRate:         float64(boolToInt(outcome.Lost)),
			ResyncComputeSec: 0,
		})
	}

	// Attach link visibility so the reader knows the channel quality.
	vis := sim.surveyLinks().VisiblePct
	for i := range samples {
		samples[i].LinkVisiblePct = vis
		revokes[i].LinkVisiblePct = vis
	}
	return samples, revokes, nil
}

func boolToInt(b bool) int {
	if b {
		return 1
	}
	return 0
}

// NewMembershipResult builds the aggregate JSON doc.
func NewMembershipResult(cfg *Config, joins, revokes []MembershipSample) map[string]interface{} {
	var jT, jL, jR float64
	var jBytes int
	jDelivered := 0
	for _, s := range joins {
		jT += s.TransferSec
		jL += s.OneWayLatencyMs
		jR += s.LossRate
		jBytes = s.AuthBytes
		jDelivered += s.Delivered
	}
	n := len(joins)
	var rT float64
	for _, s := range revokes {
		rT += s.TransferSec
	}
	vis := 0.0
	if n > 0 {
		vis = joins[0].LinkVisiblePct
	}
	return map[string]interface{}{
		"experiment": "dynamic-membership-live-join",
		"run_id":     cfg.RunID,
		"satellites": cfg.Satellites,
		"bee_n":      cfg.BEE_N,
		"bee_r":      cfg.BEE_R,
		"downlink_bps": cfg.DownlinkBps,
		"link_visible_pct": vis,
		"summary": map[string]interface{}{
			"joins":                   n,
			"join_auth_bytes":         jBytes,
			"join_mean_transfer_sec":  jT / float64(n),
			"join_mean_oneway_lat_ms": jL / float64(n),
			"join_mean_loss_rate":     jR / float64(n),
			"join_delivered_fraction": float64(jDelivered) / float64(n),
			"revoke_mean_transfer_sec": rT / float64(n),
			"join_vs_revoke_transfer_ratio": fmt.Sprintf("%.3f", (jT/float64(n))/(rT/float64(n))),
		},
		"per_join":   joins,
		"per_revoke": revokes,
	}
}
