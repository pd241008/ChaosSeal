package engine

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/chaosseal/netsim/core/kinematics"
)

// intList implements flag.Value for a comma-separated list of ints.
type intList struct {
	c *[]int
}

func (il *intList) String() string {
	if il.c == nil {
		return ""
	}
	parts := make([]string, len(*il.c))
	for i, v := range *il.c {
		parts[i] = strconv.Itoa(v)
	}
	return strings.Join(parts, ",")
}

func (il *intList) Set(s string) error {
	for _, p := range strings.Split(s, ",") {
		p = strings.TrimSpace(p)
		if p == "" {
			continue
		}
		v, err := strconv.Atoi(p)
		if err != nil {
			return err
		}
		*il.c = append(*il.c, v)
	}
	return nil
}

// Config captures the full parameter set of a simulation run. Every field is
// recorded in the results JSON so a run can be reproduced bit-for-bit.
type Config struct {
	RunID       string   `json:"run_id"`
	Seed        int64    `json:"rng_seed"`
	CorruptionTest bool   `json:"corruption_test,omitempty"`
	MembershipTest bool   `json:"membership_test,omitempty"`
	MembershipJoins int   `json:"membership_joins,omitempty"`
	CorruptBitPositions []int `json:"corrupt_bit_positions,omitempty"`
	PacketsPerEpoch     uint32 `json:"packets_per_epoch,omitempty"`
	MaxCorruptionEpochs int    `json:"max_corruption_epochs,omitempty"`
	Satellites  int      `json:"satellites"`
	GroundLat   float64  `json:"ground_lat_deg"`
	GroundLon   float64  `json:"ground_lon_deg"`
	AltitudeKm  float64  `json:"altitude_km"`
	MinElevDeg  float64  `json:"min_elevation_deg"`
	DurationSec float64  `json:"duration_sec"`
	Baselines   []string `json:"baselines"`
	BEE_N           int      `json:"bee_n"`
	BEE_R           int      `json:"bee_r"`
	PayloadBytes    int      `json:"payload_bytes"`
	LossRateOverride float64 `json:"loss_rate_override,omitempty"` // 0 = use Gilbert-Elliott
	CommitIntervalN int      `json:"commit_interval_n"` // HMAC verify every N packets (0 = every packet)
	RustCLI         string   `json:"rust_cli"`
	ResultsDir      string   `json:"results_dir"`
	DownlinkBps     float64  `json:"downlink_bandwidth_bps"`
	LyapunovSteps   int      `json:"lyapunov_steps"`

	Loss kinematics.LossConfig `json:"loss_model"`
}

// DefaultConfig returns the parameter set used for the paper runs.
func DefaultConfig() *Config {
	return &Config{
		RunID:       "manual",
		Seed:        12345,
		Satellites:  24,
		GroundLat:   37.7749,
		GroundLon:   -122.4194,
		AltitudeKm:  550,
		MinElevDeg:  10,
		DurationSec: 1200,
		Baselines:   []string{"chaosseal", "tls13", "bpsec"},
		BEE_N:         1024,
		BEE_R:         8,
		PayloadBytes:  1024,
		CommitIntervalN: 0, // 0 = verify every packet
		RustCLI:       "cargo run --quiet --manifest-path core/Cargo.toml --",
		ResultsDir:    "results",
		DownlinkBps:   50e6, // 50 Mbps downlink, Starlink-class
		LyapunovSteps: 10000,
		Loss: kinematics.LossConfig{
			PGoodToBad: 0.05,
			PBadToGood: 0.35,
			GoodLoss:   0.001,
			BadLoss:    0.25,
		},
	}
}

// ParseConfig builds a Config from command-line flags and an optional JSON
// config file. Explicit flags always override file values.
func ParseConfig(args []string) (*Config, error) {
	cfg := DefaultConfig()

	var (
		configPath     string
		baselines      string
		packetsPerEpoch int
	)
	fs := flag.NewFlagSet("netsim", flag.ContinueOnError)
	fs.StringVar(&configPath, "config", "", "path to a JSON config file (optional)")
	fs.StringVar(&cfg.RunID, "run-id", cfg.RunID, "run ID (defaults to 'manual')")
	fs.Int64Var(&cfg.Seed, "seed", cfg.Seed, "64-bit RNG seed")
	fs.IntVar(&cfg.Satellites, "satellites", cfg.Satellites, "number of LEO satellites")
	fs.Float64Var(&cfg.GroundLat, "lat", cfg.GroundLat, "ground station latitude (deg)")
	fs.Float64Var(&cfg.GroundLon, "lon", cfg.GroundLon, "ground station longitude (deg)")
	fs.Float64Var(&cfg.AltitudeKm, "altitude", cfg.AltitudeKm, "satellite orbital altitude (km)")
	fs.Float64Var(&cfg.MinElevDeg, "min-elev", cfg.MinElevDeg, "minimum elevation for a visible link (deg)")
	fs.Float64Var(&cfg.DurationSec, "duration", cfg.DurationSec, "simulated duration (seconds)")
	fs.StringVar(&baselines, "baselines", strings.Join(cfg.Baselines, ","), "comma-separated baselines to run")
	fs.IntVar(&cfg.BEE_N, "bee-n", cfg.BEE_N, "BEE key-tree size N")
	fs.IntVar(&cfg.BEE_R, "bee-r", cfg.BEE_R, "number of revoked receivers R")
	fs.IntVar(&cfg.PayloadBytes, "payload-bytes", cfg.PayloadBytes, "application payload size in bytes")
	fs.Float64Var(&cfg.LossRateOverride, "loss-rate", cfg.LossRateOverride, "fixed packet loss rate (0 = use Gilbert-Elliott)")
	fs.IntVar(&cfg.CommitIntervalN, "commit-interval", cfg.CommitIntervalN, "HMAC verify every N packets (0 = every packet)")
	fs.BoolVar(&cfg.CorruptionTest, "corruption-test", false, "run the single-bit corruption detection experiment (no network sweep)")
	fs.BoolVar(&cfg.MembershipTest, "membership-test", false, "run the dynamic-membership live-join scenario (no network sweep)")
	fs.IntVar(&cfg.MembershipJoins, "membership-joins", 8, "number of join broadcasts to simulate in the membership test")
	fs.Var(&intList{c: &cfg.CorruptBitPositions}, "corrupt-bit-positions", "comma-separated bit positions to corrupt (default: all 0..255)")
	fs.IntVar(&packetsPerEpoch, "packets-per-epoch", 64, "packets to check per epoch in corruption test")
	fs.IntVar(&cfg.MaxCorruptionEpochs, "max-corruption-epochs", 256, "give-up epoch threshold in corruption test")
	fs.StringVar(&cfg.RustCLI, "rust-cli", cfg.RustCLI, "command used to invoke the Rust core CLI")
	fs.StringVar(&cfg.ResultsDir, "results-dir", cfg.ResultsDir, "directory for result JSON files")
	if err := fs.Parse(args); err != nil {
		return nil, err
	}
	cfg.PacketsPerEpoch = uint32(packetsPerEpoch)

	if configPath != "" {
		data, err := os.ReadFile(configPath)
		if err != nil {
			return nil, fmt.Errorf("reading config file: %w", err)
		}
		if err := json.Unmarshal(data, cfg); err != nil {
			return nil, fmt.Errorf("parsing config file: %w", err)
		}
	}

	if baselines != "" {
		parts := strings.Split(baselines, ",")
		cfg.Baselines = cfg.Baselines[:0]
		for _, p := range parts {
			if p = strings.TrimSpace(p); p != "" {
				cfg.Baselines = append(cfg.Baselines, p)
			}
		}
	}

	if err := cfg.Validate(); err != nil {
		return nil, err
	}
	return cfg, nil
}

// Validate enforces physical and logical constraints on the config.
func (c *Config) Validate() error {
	if c.Satellites < 1 {
		return fmt.Errorf("satellites must be >= 1, got %d", c.Satellites)
	}
	if c.AltitudeKm < 150 {
		return fmt.Errorf("altitude %v km is below the LEO floor (150 km)", c.AltitudeKm)
	}
	if c.MinElevDeg < 0 || c.MinElevDeg > 90 {
		return fmt.Errorf("min elevation must be in [0,90], got %v", c.MinElevDeg)
	}
	if c.DurationSec <= 0 {
		return fmt.Errorf("duration must be positive, got %v", c.DurationSec)
	}
	if c.BEE_R > c.BEE_N {
		return fmt.Errorf("BEE_R (%d) cannot exceed BEE_N (%d)", c.BEE_R, c.BEE_N)
	}
	for _, b := range c.Baselines {
		switch b {
		case "chaosseal", "counter", "tls13", "bpsec":
		default:
			return fmt.Errorf("unknown baseline %q (want chaosseal, counter, tls13, or bpsec)", b)
		}
	}
	return nil
}

// AbsResultsDir returns the absolute results directory.
func (c *Config) AbsResultsDir() (string, error) {
	abs, err := filepath.Abs(c.ResultsDir)
	if err != nil {
		return "", err
	}
	return abs, nil
}
