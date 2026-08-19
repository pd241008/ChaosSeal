package client

import (
	"context"
	"testing"
)

func TestRustCoreLyapunov(t *testing.T) {
	client := &RustCoreClient{Command: fakeRustCLI(t)}
	res, err := client.Lyapunov(context.Background(), 3, 1.0, 1.0, 0.1, 0.5, 1000)
	if err != nil {
		t.Fatalf("Lyapunov: %v", err)
	}
	if res.Lambda1 != 0.1234 {
		t.Errorf("lambda1 = %v, want 0.1234", res.Lambda1)
	}
}

func TestRustCoreBeeSize(t *testing.T) {
	client := &RustCoreClient{Command: fakeRustCLI(t)}
	res, err := client.BeeSize(context.Background(), 1024, 8)
	if err != nil {
		t.Fatalf("BeeSize: %v", err)
	}
	if res.CiphertextSizeMin != 224 {
		t.Errorf("ciphertext size = %d, want 224", res.CiphertextSizeMin)
	}
}

func TestRustCoreErrorPropagates(t *testing.T) {
	// Point at a script that fails so we can assert error handling.
	client := &RustCoreClient{Command: fakeRustCLI(t)}
	if _, err := client.Run(context.Background(), "bogus"); err == nil {
		t.Fatal("Run(bogus) = nil, want error for unknown subcommand")
	}
}
