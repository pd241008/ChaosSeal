package main

import (
	"strings"
	"testing"
	"time"
)

func TestTLS13BaselineNoLatency(t *testing.T) {
	noLatency := func() time.Duration { return 0 }
	res, err := RunTLS13Baseline(noLatency, make([]byte, 1024))
	if err != nil {
		t.Fatalf("RunTLS13Baseline: %v", err)
	}
	if res.HandshakeSec <= 0 {
		t.Errorf("handshake sec = %v, want > 0", res.HandshakeSec)
	}
	if res.BytesSent == 0 || res.BytesReceived == 0 {
		t.Errorf("expected bytes both ways, got sent=%d recv=%d", res.BytesSent, res.BytesReceived)
	}
	if !strings.HasPrefix(res.CipherSuite, "TLS_AES") {
		t.Errorf("cipher suite = %q, want a TLS 1.3 (TLS_AES_*) suite", res.CipherSuite)
	}
}

func TestTLS13BaselineLatencyAddsDelay(t *testing.T) {
	fast := func() time.Duration { return 0 }
	slow := func() time.Duration { return 5 * time.Millisecond }

	r1, err := RunTLS13Baseline(fast, make([]byte, 1024))
	if err != nil {
		t.Fatal(err)
	}
	r2, err := RunTLS13Baseline(slow, make([]byte, 1024))
	if err != nil {
		t.Fatal(err)
	}
	if r2.HandshakeSec <= r1.HandshakeSec {
		t.Errorf("handshake with latency (%v s) not slower than without (%v s)", r2.HandshakeSec, r1.HandshakeSec)
	}
}

func TestTLS13BaselineRoundTripPayload(t *testing.T) {
	payload := []byte("hello chaosseal baseline")
	res, err := RunTLS13Baseline(func() time.Duration { return 0 }, payload)
	if err != nil {
		t.Fatalf("RunTLS13Baseline: %v", err)
	}
	if res.AppPayloadSec < 0 {
		t.Errorf("negative app payload time: %v", res.AppPayloadSec)
	}
}
