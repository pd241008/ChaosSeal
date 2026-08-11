package main

import (
	"context"
	"testing"
)

func TestSimulationRunsAllBaselines(t *testing.T) {
	cfg := DefaultConfig()
	cfg.Seed = 2024
	cfg.Satellites = 4
	cfg.DurationSec = 10
	cfg.RustCLI = fakeRustCLI(t)
	cfg.Baselines = []string{"chaosseal", "tls13", "bpsec"}

	sim := NewSimulation(cfg)
	res, err := sim.Run(context.Background())
	if err != nil {
		t.Fatalf("Simulation.Run: %v", err)
	}

	for _, name := range []string{"chaosseal", "tls13", "bpsec"} {
		if _, ok := res.Baselines[name]; !ok {
			t.Errorf("missing baseline result %q", name)
		}
	}

	if res.LinkStats == nil {
		t.Fatal("link stats missing")
	}
	if res.LinkStats.Satellites != 4 {
		t.Errorf("satellites in stats = %d, want 4", res.LinkStats.Satellites)
	}
	if res.LinkStats.VisiblePct < 0 || res.LinkStats.VisiblePct > 100 {
		t.Errorf("visible %% = %v out of range", res.LinkStats.VisiblePct)
	}
	if res.LinkStats.MeanLatencyMs <= 0 {
		t.Errorf("mean latency = %v, want > 0", res.LinkStats.MeanLatencyMs)
	}
	if len(res.Events) == 0 {
		t.Error("no events recorded")
	}
}

func TestSimulationDeterminism(t *testing.T) {
	mk := func() *Simulation {
		cfg := DefaultConfig()
		cfg.Seed = 555
		cfg.Satellites = 3
		cfg.DurationSec = 8
		cfg.RustCLI = fakeRustCLI(t)
		cfg.Baselines = []string{"bpsec", "tls13"}
		return NewSimulation(cfg)
	}

	a, errA := mk().Run(context.Background())
	if errA != nil {
		t.Fatalf("run A: %v", errA)
	}
	b, errB := mk().Run(context.Background())
	if errB != nil {
		t.Fatalf("run B: %v", errB)
	}

	if a.LinkStats.VisiblePct != b.LinkStats.VisiblePct {
		t.Errorf("visible pct not deterministic: %v vs %v", a.LinkStats.VisiblePct, b.LinkStats.VisiblePct)
	}
	if a.LinkStats.LossRate != b.LinkStats.LossRate {
		t.Errorf("loss rate not deterministic: %v vs %v", a.LinkStats.LossRate, b.LinkStats.LossRate)
	}
	if len(a.Events) != len(b.Events) {
		t.Errorf("event counts differ: %d vs %d", len(a.Events), len(b.Events))
	}
}

func TestSimulationChaosSealInvokesRust(t *testing.T) {
	cfg := DefaultConfig()
	cfg.Seed = 99
	cfg.Satellites = 2
	cfg.DurationSec = 5
	cfg.RustCLI = fakeRustCLI(t)
	cfg.Baselines = []string{"chaosseal"}

	res, err := NewSimulation(cfg).Run(context.Background())
	if err != nil {
		t.Fatalf("run: %v", err)
	}
	cs, ok := res.Baselines["chaosseal"].(map[string]interface{})
	if !ok {
		t.Fatalf("chaosseal baseline not a map: %T", res.Baselines["chaosseal"])
	}
	bee, ok := cs["bee"].(*BeeSizeResult)
	if !ok {
		t.Fatalf("bee result wrong type: %T", cs["bee"])
	}
	if bee.CiphertextSizeMin != 224 {
		t.Errorf("bee ciphertext = %d, want 224 from fake CLI", bee.CiphertextSizeMin)
	}
}
