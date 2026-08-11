package main

import (
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

// RunResult is the single source of truth for every simulation run. It is
// written to /results/<run_id>.json and read by the Python analysis and the
// dashboard; neither may recompute protocol logic from it.
type RunResult struct {
	RunID      string                   `json:"run_id"`
	GitCommit  string                   `json:"git_commit"`
	RNGSeed    int64                    `json:"rng_seed"`
	StartedAt  string                   `json:"started_at_utc"`
	Parameters *Config                  `json:"parameters"`
	LinkStats  *LinkStats               `json:"link_stats"`
	Baselines  map[string]interface{}   `json:"baselines"`
	Events     []Event                  `json:"events"`
}

// Event records one discrete occurrence during the run.
type Event struct {
	TimeSec   float64 `json:"t_sec"`
	Type      string  `json:"type"`
	Satellite int     `json:"satellite,omitempty"`
	Detail    string  `json:"detail,omitempty"`
	Value     float64 `json:"value,omitempty"`
}

// LinkStats aggregates the geometric and channel statistics over the run.
type LinkStats struct {
	Satellites    int     `json:"satellites"`
	VisiblePct    float64 `json:"visible_pct"`
	MeanLatencyMs float64 `json:"mean_latency_ms"`
	MaxLatencyMs  float64 `json:"max_latency_ms"`
	LossRate      float64 `json:"loss_rate"`
	BurstCount    int     `json:"burst_count"`
	MaxBurstLen   int     `json:"max_burst_len"`
}

// NewRunResult initializes a result for the given config.
func NewRunResult(cfg *Config) *RunResult {
	return &RunResult{
		RunID:      cfg.RunID,
		GitCommit:  gitCommit(),
		RNGSeed:    cfg.Seed,
		StartedAt:  time.Now().UTC().Format(time.RFC3339),
		Parameters: cfg,
		Baselines:  map[string]interface{}{},
		Events:     []Event{},
	}
}

// Write persists the run result to <results_dir>/<run_id>.json.
func (r *RunResult) Write(resultsDir string) (string, error) {
	if err := os.MkdirAll(resultsDir, 0o755); err != nil {
		return "", err
	}
	data, err := json.MarshalIndent(r, "", "  ")
	if err != nil {
		return "", err
	}
	path := filepath.Join(resultsDir, r.RunID+".json")
	if err := os.WriteFile(path, data, 0o644); err != nil {
		return "", err
	}
	return path, nil
}

// gitCommit returns the full HEAD sha of the enclosing repository, or a
// sentinel when git is unavailable (e.g. tests).
func gitCommit() string {
	cmd := exec.Command("git", "rev-parse", "HEAD")
	out, err := cmd.Output()
	if err != nil {
		return "unknown"
	}
	return strings.TrimSpace(string(out))
}

// ResolveResultsDir returns the absolute results directory, resolving
// relative paths against the repository root so `--results-dir results`
// lands at the monorepo's /results regardless of CWD.
func ResolveResultsDir(path string) (string, error) {
	if filepath.IsAbs(path) {
		return path, nil
	}
	root, err := repoRoot()
	if err == nil {
		return filepath.Join(root, path), nil
	}
	abs, err := filepath.Abs(path)
	if err != nil {
		return "", err
	}
	return abs, nil
}

func repoRoot() (string, error) {
	cmd := exec.Command("git", "rev-parse", "--show-toplevel")
	out, err := cmd.Output()
	if err != nil {
		return "", err
	}
	return strings.TrimSpace(string(out)), nil
}
