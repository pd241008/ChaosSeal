package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestParseConfigDefaults(t *testing.T) {
	cfg, err := ParseConfig(nil)
	if err != nil {
		t.Fatalf("ParseConfig(nil): %v", err)
	}
	if cfg.Seed != 12345 {
		t.Errorf("default seed = %d, want 12345", cfg.Seed)
	}
	if cfg.Satellites != 24 {
		t.Errorf("default satellites = %d, want 24", cfg.Satellites)
	}
	if len(cfg.Baselines) != 3 {
		t.Errorf("default baselines = %v, want 3 baselines", cfg.Baselines)
	}
	if cfg.DownlinkBps != 50e6 {
		t.Errorf("default downlink = %v, want 50e6", cfg.DownlinkBps)
	}
}

func TestParseConfigFlagOverrides(t *testing.T) {
	cfg, err := ParseConfig([]string{"--seed", "42", "--satellites", "4", "--baselines", "tls13,bpsec"})
	if err != nil {
		t.Fatalf("ParseConfig: %v", err)
	}
	if cfg.Seed != 42 {
		t.Errorf("seed = %d, want 42", cfg.Seed)
	}
	if cfg.Satellites != 4 {
		t.Errorf("satellites = %d, want 4", cfg.Satellites)
	}
	if len(cfg.Baselines) != 2 || cfg.Baselines[0] != "tls13" {
		t.Errorf("baselines = %v, want [tls13 bpsec]", cfg.Baselines)
	}
}

func TestParseConfigFile(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "cfg.json")
	data := []byte(`{"rng_seed": 999, "satellites": 12, "downlink_bandwidth_bps": 100000000}`)
	if err := os.WriteFile(path, data, 0o644); err != nil {
		t.Fatal(err)
	}
	cfg, err := ParseConfig([]string{"--config", path})
	if err != nil {
		t.Fatalf("ParseConfig with file: %v", err)
	}
	if cfg.Seed != 999 {
		t.Errorf("seed from file = %d, want 999", cfg.Seed)
	}
	if cfg.Satellites != 12 {
		t.Errorf("satellites from file = %d, want 12", cfg.Satellites)
	}
	if cfg.DownlinkBps != 100000000 {
		t.Errorf("downlink from file = %v, want 1e8", cfg.DownlinkBps)
	}
}

func TestValidate(t *testing.T) {
	cases := []struct {
		name string
		mut  func(*Config)
	}{
		{"zero satellites", func(c *Config) { c.Satellites = 0 }},
		{"altitude too low", func(c *Config) { c.AltitudeKm = 100 }},
		{"bad baseline", func(c *Config) { c.Baselines = []string{"nope"} }},
		{"revoked > total", func(c *Config) { c.BEE_R, c.BEE_N = 100, 10 }},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			cfg := DefaultConfig()
			tc.mut(cfg)
			if err := cfg.Validate(); err == nil {
				t.Errorf("Validate() = nil, want error for %q", tc.name)
			}
		})
	}
}
