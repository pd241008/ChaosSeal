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
	"fmt"
	"os"
	"time"
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintf(os.Stderr, "netsim: %v\n", err)
		os.Exit(1)
	}
}

func run(args []string) error {
	cfg, err := ParseConfig(args)
	if err != nil {
		return err
	}

	resultsDir, err := ResolveResultsDir(cfg.ResultsDir)
	if err != nil {
		return err
	}

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Minute)
	defer cancel()

	sim := NewSimulation(cfg)
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
