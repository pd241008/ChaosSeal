package main

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

func epoch() time.Time { return time.Now() }

// fakeRustCLI writes a small shell script that emulates the Rust core CLI's
// JSON output so tests can exercise RustCoreClient and the full simulation
// without a compiled Rust binary.
func fakeRustCLI(t *testing.T) string {
	t.Helper()
	dir := t.TempDir()
	path := filepath.Join(dir, "chaosseal-fake")
	script := `#!/bin/sh
case "$1" in
  lyapunov)
    echo '{"success": true, "output": {"lambda1": 0.1234, "dt_bound": 0.0811, "parameters": {}}}'
    ;;
  bee-size)
    echo '{"success": true, "output": {"n": 1024, "r": 8, "ciphertext_size_bytes": 224}}'
    ;;
  *)
    echo '{"success": false, "output": {"error": "unknown subcommand"}}'
    exit 1
    ;;
esac
`
	if err := os.WriteFile(path, []byte(script), 0o755); err != nil {
		t.Fatalf("writing fake rust cli: %v", err)
	}
	return path
}
