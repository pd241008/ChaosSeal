package main

import (
	"math/rand"
	"testing"
)

func TestGilbertElliottDeterminism(t *testing.T) {
	cfg := LossConfig{PGoodToBad: 0.1, PBadToGood: 0.5, GoodLoss: 0.01, BadLoss: 0.3}
	a := NewGilbertElliott(cfg, rand.New(rand.NewSource(7)))
	b := NewGilbertElliott(cfg, rand.New(rand.NewSource(7)))

	seqA := make([]bool, 1000)
	seqB := make([]bool, 1000)
	for i := range seqA {
		seqA[i] = a.Next()
		seqB[i] = b.Next()
	}
	for i := range seqA {
		if seqA[i] != seqB[i] {
			t.Fatalf("deterministic RNG diverged at step %d", i)
		}
	}
}

func TestGilbertElliottBursts(t *testing.T) {
	// When the bad state has high loss and the chain stays bad with a low
	// exit probability, losses should cluster into bursts.
	cfg := LossConfig{PGoodToBad: 0.3, PBadToGood: 0.05, GoodLoss: 0.001, BadLoss: 0.8}
	ge := NewGilbertElliott(cfg, rand.New(rand.NewSource(1)))

	losses := make([]bool, 2000)
	for i := range losses {
		losses[i] = ge.Next()
	}

	// Count distinct bursts.
	inBurst := false
	bursts := 0
	lossCount := 0
	for _, l := range losses {
		if l {
			lossCount++
			if !inBurst {
				bursts++
				inBurst = true
			}
		} else {
			inBurst = false
		}
	}
	if lossCount == 0 {
		t.Fatal("expected some losses from burst-heavy channel")
	}
	if bursts > lossCount/2 {
		t.Errorf("losses not clustered into bursts: %d losses in %d bursts", lossCount, bursts)
	}
}

func TestLinkTransmitVisibility(t *testing.T) {
	gs := &GroundStation{LatDeg: 0, LonDeg: 0}
	sat := NewSatellite(0, 550, 0, 0, 0)
	// Lossless channel: every visible packet gets through.
	ch := NewGilbertElliott(LossConfig{PGoodToBad: 0, PBadToGood: 1, GoodLoss: 0, BadLoss: 0}, rand.New(rand.NewSource(1)))
	link := NewLink(sat, gs, 10, ch, epoch())

	// Overhead at t=0: must transmit successfully.
	out := link.Transmit(0)
	if out.Lost {
		t.Error("visible overhead link dropped a packet")
	}
	if out.Progress != 1 {
		t.Error("visible packet did not complete")
	}
	if out.Latency <= 0 {
		t.Errorf("latency = %v, want positive", out.Latency)
	}
}

func TestLinkInvisibleDrops(t *testing.T) {
	gs := &GroundStation{LatDeg: 0, LonDeg: 0}
	sat := NewSatellite(0, 550, 0, 0, 0)
	ch := NewGilbertElliott(LossConfig{PGoodToBad: 0, PBadToGood: 1, GoodLoss: 0, BadLoss: 0}, rand.New(rand.NewSource(1)))
	link := NewLink(sat, gs, 10, ch, epoch())

	// Half a period later the satellite is below the horizon: must drop.
	out := link.Transmit(sat.PeriodSec / 2)
	if !out.Lost {
		t.Error("below-horizon link should drop packets")
	}
}
