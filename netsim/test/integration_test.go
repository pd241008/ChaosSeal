package test

import (
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/chaosseal/netsim/core/engine"
)

// TestFullPipelineRunsEndToEnd exercises the whole simulation pipeline through
// the public engine API: config -> run -> write JSON -> read back. It mirrors
// core/tests/kat.rs as the cross-package integration check of the monorepo.
func TestFullPipelineRunsEndToEnd(t *testing.T) {
	cfg := engine.DefaultConfig()
	cfg.RunID = "integration-test"
	cfg.Seed = 42
	cfg.Satellites = 4
	cfg.DurationSec = 10
	cfg.RustCLI = fakeRustCLI(t)
	cfg.Baselines = []string{"chaosseal", "tls13", "bpsec"}

	res, err := engine.NewSimulation(cfg).Run(context.Background())
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
	if len(res.Events) == 0 {
		t.Error("no events recorded")
	}

	dir := t.TempDir()
	path, err := res.Write(dir)
	if err != nil {
		t.Fatalf("Write: %v", err)
	}
	if filepath.Base(path) != "integration-test.json" {
		t.Errorf("result file = %q, want integration-test.json", filepath.Base(path))
	}
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("reading result: %v", err)
	}
	var back map[string]interface{}
	if err := json.Unmarshal(data, &back); err != nil {
		t.Fatalf("result JSON does not parse: %v", err)
	}
	if back["run_id"] != "integration-test" {
		t.Errorf("decoded run_id = %v, want integration-test", back["run_id"])
	}
	if _, ok := back["baselines"]; !ok {
		t.Error("decoded result missing baselines")
	}
}
