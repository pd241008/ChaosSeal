// Command netsim runs the ChaosSeal LEO network simulation and writes one
// JSON result file per run to /results.
//
// Usage:
//
//	go run . --seed <seed> --config <parameters.json> [flags]
//
// Every stochastic and geometric input is derived from --seed, so a run can be
// reproduced exactly by replaying the same flags and git commit.
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"github.com/chaosseal/netsim/core/engine"
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintf(os.Stderr, "netsim: %v\n", err)
		os.Exit(1)
	}
}

func run(args []string) error {
	cfg, err := engine.ParseConfig(args)
	if err != nil {
		return err
	}

	resultsDir, err := engine.ResolveResultsDir(cfg.ResultsDir)
	if err != nil {
		return err
	}

	// Corruption-detection experiment: inject single-bit flips and measure how
	// many epochs pass before each key-derivation mode is detected by HMAC
	// verification failure. Not a network sweep — a core-crypto benchmark.
	if cfg.CorruptionTest {
		return runCorruptionTest(cfg, resultsDir)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Minute)
	defer cancel()

	sim := engine.NewSimulation(cfg)
	result, err := sim.Run(ctx)
	if err != nil {
		return err
	}

	path, err := result.Write(resultsDir)
	if err != nil {
		return err
	}

	fmt.Printf("run_id=%s satellites=%d baselines=%v\n", result.RunID, cfg.Satellites, cfg.Baselines)
	fmt.Printf("link: visible=%.1f%% mean_latency=%.2fms loss=%.3f\n",
		result.LinkStats.VisiblePct, result.LinkStats.MeanLatencyMs, result.LinkStats.LossRate)
	for name := range result.Baselines {
		fmt.Printf("baseline %s: ok\n", name)
	}
	fmt.Printf("results -> %s\n", path)
	return nil
}

func runCorruptionTest(cfg *engine.Config, resultsDir string) error {
	runner := engine.CorruptionRunner{}
	samples := runner.RunAll(cfg.CorruptBitPositions, cfg.PacketsPerEpoch, cfg.MaxCorruptionEpochs)

	// Aggregate stats
	var counterSum int
	counterMax := 0
	var chaosSum, chaosMax float64
	keyDiffer := 0
	for _, s := range samples {
		counterSum += s.CounterEpochsUntilDetect
		if s.CounterEpochsUntilDetect > counterMax {
			counterMax = s.CounterEpochsUntilDetect
		}
		chaosSum += s.ChaosLyapunovTimescales
		if s.ChaosLyapunovTimescales > chaosMax {
			chaosMax = s.ChaosLyapunovTimescales
		}
		if s.ChaosKeyDiffers == 1 {
			keyDiffer++
		}
	}
	n := len(samples)
	doc := map[string]interface{}{
		"experiment":        "single-bit-corruption-detection",
		"run_id":            cfg.RunID,
		"n_bit_positions":   n,
		"packets_per_epoch": cfg.PacketsPerEpoch,
		"max_epochs":        cfg.MaxCorruptionEpochs,
		"haystack_bytes":    32,
		"summary": map[string]interface{}{
			"counter_mean_epochs_to_detect":  float64(counterSum) / float64(n),
			"counter_max_epochs_to_detect":   counterMax,
			"chaos_mean_divergence_lyap_timescales": chaosSum / float64(n),
			"chaos_max_divergence_lyap_timescales":  chaosMax,
			"chaos_key_differs_fraction":            float64(keyDiffer) / float64(n),
		},
		"per_bit": samples,
	}

	if err := os.MkdirAll(resultsDir, 0o755); err != nil {
		return err
	}
	data, err := json.MarshalIndent(doc, "", "  ")
	if err != nil {
		return err
	}
	path := filepath.Join(resultsDir, cfg.RunID+".json")
	if err := os.WriteFile(path, data, 0o644); err != nil {
		return err
	}
	fmt.Printf("corruption-test: ran %d bit positions -> %s\n", n, path)
	return nil
}
