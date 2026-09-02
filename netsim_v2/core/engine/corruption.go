package engine

import (
	"github.com/chaosseal/netsim/core/client"
)

// CorruptionRunner executes the single-bit-corruption detection experiment.
// It calls into the Rust core via CGO for each bit position and aggregates the
// per-mode detection latency (epochs until HMAC verification fails).
type CorruptionRunner struct{}

// CorruptionSample records detection latency for one injected bit position.
type CorruptionSample struct {
	BitPosition               int     `json:"bit_position"`
	CounterEpochsUntilDetect  int     `json:"counter_epochs_until_detect"`
	ChaosDivergenceSteps      int     `json:"chaos_divergence_steps"`
	ChaosDivergenceTimeSec    float64 `json:"chaos_divergence_time_sec"`
	ChaosLyapunovTimescales   float64 `json:"chaos_lyapunov_timescales"`
	ChaosKeyDiffers           int     `json:"chaos_key_differs"`
}

// RunAll runs the detection experiment for every requested bit position.
// Default bit positions = 0..255 (all bits of the 32-byte seed) and a selection
// of the pendulum initial-state bits.
func (r *CorruptionRunner) RunAll(bitPositions []int, packetsPerEpoch uint32, maxEpochs int) []CorruptionSample {
	if len(bitPositions) == 0 {
		bitPositions = make([]int, 256)
		for i := range bitPositions {
			bitPositions[i] = i
		}
	}
	if packetsPerEpoch == 0 {
		packetsPerEpoch = 64
	}
	if maxEpochs == 0 {
		maxEpochs = 256
	}

	out := make([]CorruptionSample, 0, len(bitPositions))
	for _, bp := range bitPositions {
		res := client.CorruptionTest(bp, packetsPerEpoch, maxEpochs)
		out = append(out, CorruptionSample{
			BitPosition:              bp,
			CounterEpochsUntilDetect: res.CounterEpochsUntilDetect,
			ChaosDivergenceSteps:     res.ChaosDivergenceSteps,
			ChaosDivergenceTimeSec:   res.ChaosDivergenceTimeSec,
			ChaosLyapunovTimescales:  res.ChaosLyapunovTimescales,
			ChaosKeyDiffers:          res.ChaosKeyDiffers,
		})
	}
	return out
}
