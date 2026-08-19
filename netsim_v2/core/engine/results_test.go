package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestNewRunResult(t *testing.T) {
	cfg := DefaultConfig()
	cfg.RunID = "test-run"
	r := NewRunResult(cfg)
	if r.GitCommit == "" {
		t.Error("git commit is empty")
	}
	if r.RunID != "test-run" {
		t.Errorf("run id = %q, want test-run", r.RunID)
	}
}

func TestWriteCreatesJSONFile(t *testing.T) {
	cfg := DefaultConfig()
	cfg.RunID = "test-write"
	r := NewRunResult(cfg)
	r.Baselines["tls13"] = map[string]interface{}{"handshake_sec": 0.1}
	r.Events = append(r.Events, Event{TimeSec: 1.0, Type: "link_up", Satellite: 3})

	dir := t.TempDir()
	path, err := r.Write(dir)
	if err != nil {
		t.Fatalf("Write: %v", err)
	}
	wantPath := filepath.Join(dir, "test-write.json")
	if path != wantPath {
		t.Errorf("path = %q, want %q", path, wantPath)
	}
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var back RunResult
	if err := json.Unmarshal(data, &back); err != nil {
		t.Fatalf("result JSON does not parse: %v", err)
	}
	if back.RunID != "test-write" {
		t.Errorf("decoded run id = %q, want test-write", back.RunID)
	}
	if len(back.Events) != 1 {
		t.Errorf("decoded events = %d, want 1", len(back.Events))
	}
}

func TestResolveResultsDir(t *testing.T) {
	dir, err := ResolveResultsDir("results")
	if err != nil {
		t.Fatalf("ResolveResultsDir: %v", err)
	}
	// In the repo, this must resolve to <repo>/results.
	root, err := repoRoot()
	if err != nil {
		t.Skip("not in a git repo")
	}
	want := filepath.Join(root, "results")
	if dir != want {
		t.Errorf("resolved dir = %q, want %q", dir, want)
	}
}
