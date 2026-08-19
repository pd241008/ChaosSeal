package client

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"os/exec"
	"strings"
	"time"
)

// RustCoreClient drives the Rust protocol core via its CLI subprocess. The
// core emits a single JSON object on stdout:
//
//	{"success": true, "output": { ... }}
//
// See core/src/bin/cli.rs for the exact schema.
type RustCoreClient struct {
	// Command is the space-separated program + leading args used to invoke
	// the core CLI, e.g. "cargo run --quiet --manifest-path core/Cargo.toml --"
	// or "/path/to/chaosseal".
	Command string

	// Timeout bounds a single CLI invocation. Defaults to 60s.
	Timeout time.Duration
}

type coreEnvelope struct {
	Success bool            `json:"success"`
	Output  json.RawMessage `json:"output"`
}

// LyapunovResult mirrors the core CLI's `lyapunov` output.
type LyapunovResult struct {
	Lambda1 float64 `json:"lambda1"`
	DTBound float64 `json:"dt_bound"`
}

// BeeSizeResult mirrors the core CLI's `beesize` output.
type BeeSizeResult struct {
	N                 int `json:"n"`
	R                 int `json:"r"`
	CiphertextSizeMin int `json:"ciphertext_size_bytes"`
}

// Run invokes the core CLI with the given subcommand and raw args, returning
// the parsed "output" object.
func (c *RustCoreClient) Run(ctx context.Context, args ...string) (json.RawMessage, error) {
	if c.Command == "" {
		return nil, fmt.Errorf("rust core command is empty")
	}
	argv := strings.Fields(c.Command)
	argv = append(argv, args...)

	timeout := c.Timeout
	if timeout == 0 {
		timeout = 60 * time.Second
	}
	ctx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	var stdout, stderr bytes.Buffer
	cmd := exec.CommandContext(ctx, argv[0], argv[1:]...)
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	if err := cmd.Run(); err != nil {
		return nil, fmt.Errorf("rust core %s failed: %w (stderr: %s)", strings.Join(args, " "), err, trim(stderr.String()))
	}

	var env coreEnvelope
	if err := json.Unmarshal(stdout.Bytes(), &env); err != nil {
		return nil, fmt.Errorf("parsing rust core stdout: %w\nstdout: %s\nstderr: %s", err, trim(stdout.String()), trim(stderr.String()))
	}
	if !env.Success {
		return nil, fmt.Errorf("rust core reported failure for %s: %s", strings.Join(args, " "), trim(string(env.Output)))
	}
	return env.Output, nil
}

// Lyapunov runs the core's Lyapunov exponent estimator.
func (c *RustCoreClient) Lyapunov(ctx context.Context, pendulums int, mass, length, damping, coupling float64, steps int) (*LyapunovResult, error) {
	args := []string{
		"lyapunov",
		"--pendulums", fmt.Sprint(pendulums),
		"--mass", fmt.Sprint(mass),
		"--length", fmt.Sprint(length),
		"--damping", fmt.Sprint(damping),
		"--coupling", fmt.Sprint(coupling),
		"--steps", fmt.Sprint(steps),
	}
	raw, err := c.Run(ctx, args...)
	if err != nil {
		return nil, err
	}
	var r LyapunovResult
	if err := json.Unmarshal(raw, &r); err != nil {
		return nil, fmt.Errorf("decoding lyapunov output: %w", err)
	}
	return &r, nil
}

// BeeSize runs the core's BEE ciphertext-size estimator.
func (c *RustCoreClient) BeeSize(ctx context.Context, n, r int) (*BeeSizeResult, error) {
	args := []string{"bee-size", "--n", fmt.Sprint(n), "--r", fmt.Sprint(r)}
	raw, err := c.Run(ctx, args...)
	if err != nil {
		return nil, err
	}
	var res BeeSizeResult
	if err := json.Unmarshal(raw, &res); err != nil {
		return nil, fmt.Errorf("decoding beesize output: %w", err)
	}
	return &res, nil
}

func trim(s string) string {
	s = strings.TrimSpace(s)
	if len(s) > 500 {
		return s[:500] + "..."
	}
	return s
}
