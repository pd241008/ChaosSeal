package main

import (
	"math/rand"
	"time"
)

// GilbertElliott is a two-state Markov channel model (RFC-style burst loss).
// State 0 = "good", state 1 = "bad" (burst). Losses are i.i.d. within each
// state with state-dependent probability.
type GilbertElliott struct {
	PGoodToBad float64
	PBadToGood float64
	GoodLoss   float64
	BadLoss    float64

	rng   *rand.Rand
	state int
}

// NewGilbertElliott constructs the channel model with a seeded RNG.
func NewGilbertElliott(c LossConfig, rng *rand.Rand) *GilbertElliott {
	return &GilbertElliott{
		PGoodToBad: c.PGoodToBad,
		PBadToGood: c.PBadToGood,
		GoodLoss:   c.GoodLoss,
		BadLoss:    c.BadLoss,
		rng:        rng,
	}
}

// Next advances the channel one packet step and reports whether the packet is
// lost. The channel state transition happens before the loss draw, matching
// the usual discrete-time Markov interpretation.
func (ge *GilbertElliott) Next() bool {
	if ge.rng.Float64() < ge.transitionProb() {
		ge.state = 1 - ge.state
	}
	p := ge.GoodLoss
	if ge.state == 1 {
		p = ge.BadLoss
	}
	return ge.rng.Float64() < p
}

// InBurst reports whether the channel is currently in the bad (burst) state.
func (ge *GilbertElliott) InBurst() bool {
	return ge.state == 1
}

// BurstLength returns the number of consecutive lost packets from the current
// state, advancing the RNG without altering the Markov state. Used to model
// correlated loss epochs.
func (ge *GilbertElliott) BurstLength(maxLen int) int {
	n := 0
	for n < maxLen && ge.rng.Float64() < ge.BadLoss {
		n++
	}
	return n
}

func (ge *GilbertElliott) transitionProb() float64 {
	if ge.state == 0 {
		return ge.PGoodToBad
	}
	return ge.PBadToGood
}

// PacketOutcome is the result of attempting to send one packet over a link.
type PacketOutcome struct {
	Lost     bool
	InBurst  bool
	At       time.Time
	Latency  time.Duration
	Progress int // 0 = not received, 1 = received
}

// Link couples a satellite-ground station geometry with a Gilbert-Elliott
// loss process. It is the unit the simulation transmits over.
type Link struct {
	Sat       *Satellite
	GS        *GroundStation
	MinElev   float64
	Channel   *GilbertElliott
	StartTime time.Time
}

// NewLink builds a link for the given satellite at the simulation epoch.
func NewLink(sat *Satellite, gs *GroundStation, minElev float64, ch *GilbertElliott, epoch time.Time) *Link {
	return &Link{Sat: sat, GS: gs, MinElev: minElev, Channel: ch, StartTime: epoch}
}

// VisibleAt reports link visibility at a wall-clock offset from epoch.
func (l *Link) VisibleAt(offsetSec float64) bool {
	return l.GS.IsVisible(l.Sat, l.MinElev, offsetSec)
}

// LatencyAt returns the one-way propagation delay at the given offset.
func (l *Link) LatencyAt(offsetSec float64) time.Duration {
	sec := l.GS.OneWayLatencySec(l.Sat, offsetSec)
	return time.Duration(sec * float64(time.Second))
}

// Transmit sends one packet at offsetSec. It applies elevation-dependent
// latency and the Gilbert-Elliott loss process. Returns the outcome.
func (l *Link) Transmit(offsetSec float64) PacketOutcome {
	visible := l.VisibleAt(offsetSec)
	latency := l.LatencyAt(offsetSec)
	lost := l.Channel.Next()
	if !visible || lost {
		return PacketOutcome{Lost: true, InBurst: l.Channel.InBurst(), At: l.StartTime, Latency: latency}
	}
	return PacketOutcome{Lost: false, InBurst: l.Channel.InBurst(), At: l.StartTime.Add(latency), Latency: latency, Progress: 1}
}
