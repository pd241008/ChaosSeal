package client

/*
#cgo LDFLAGS: ${SRCDIR}/../../../core_v2/target/release/libchaosseal_core.a -lpthread -ldl -lm
#include <stdint.h>
#include <stdlib.h>

typedef struct {
    uintptr_t crypto_overhead;
} CEpochCryptoResult;

typedef struct {
    uintptr_t crypto_overhead;
    uint32_t counter_value;
} CCounterResult;

typedef struct {
    uintptr_t counter_epochs_until_detect;
    uintptr_t chaos_divergence_steps;
    double chaos_divergence_time_sec;
    double chaos_lyapunov_timescales;
    int32_t chaos_key_differs;
} CCorruptionTestResult;

void chaosseal_epoch_keygen();
CEpochCryptoResult chaosseal_epoch_crypto(const uint8_t* payload, uintptr_t payload_len);
void chaosseal_counter_epoch_keygen();
CCounterResult chaosseal_counter_epoch_crypto(const uint8_t* payload, uintptr_t payload_len);
CCorruptionTestResult chaosseal_corruption_test(uintptr_t corrupt_bit_pos, uint32_t packets_per_epoch, uintptr_t max_epochs);
*/
import "C"
import "unsafe"

// EpochCryptoResult represents the sizes returned by the simulated epoch crypto.
type EpochCryptoResult struct {
	CryptoOverhead int
}

// EpochCrypto invokes the Rust FFI function to simulate data transmission crypto
// for a single epoch (HKDF key derivation, AES-256-CTR, and HMAC-SHA256).
// It returns the lengths of the ciphertext and HMAC, taking real wall-clock time
// executing the cryptography.
func EpochCrypto(payload []byte) EpochCryptoResult {
	var cPayload *C.uint8_t
	if len(payload) > 0 {
		cPayload = (*C.uint8_t)(unsafe.Pointer(&payload[0]))
	}
	cLen := (C.uintptr_t)(len(payload))

	res := C.chaosseal_epoch_crypto(cPayload, cLen)

	return EpochCryptoResult{
		CryptoOverhead: int(res.crypto_overhead),
	}
}

// EpochKeyGen invokes the Rust FFI function to simulate the background chaotic
// key derivation for a 1200s epoch (120,000 steps of RK4 integration + HKDF).
// It takes real wall-clock time proportional to the physical integration overhead.
func EpochKeyGen() {
	C.chaosseal_epoch_keygen()
}

// CounterEpochKeyGen invokes the counter-mode key generation (instant, no chaotic simulation).
func CounterEpochKeyGen() {
	C.chaosseal_counter_epoch_keygen()
}

// CounterEpochCrypto invokes counter-mode per-packet encryption via Rust FFI.
func CounterEpochCrypto(payload []byte) (EpochCryptoResult, uint32) {
	var cPayload *C.uint8_t
	if len(payload) > 0 {
		cPayload = (*C.uint8_t)(unsafe.Pointer(&payload[0]))
	}
	cLen := (C.uintptr_t)(len(payload))
	res := C.chaosseal_counter_epoch_crypto(cPayload, cLen)
	return EpochCryptoResult{
		CryptoOverhead: int(res.crypto_overhead),
	}, uint32(res.counter_value)
}

// CorruptionTestResult holds the output of a single-bit-corruption detection experiment.
type CorruptionTestResult struct {
	CounterEpochsUntilDetect int
	ChaosDivergenceSteps     int
	ChaosDivergenceTimeSec   float64
	ChaosLyapunovTimescales  float64
	ChaosKeyDiffers          int
}

// CorruptionTest injects a single-bit flip and measures detection latency.
func CorruptionTest(corruptBitPos int, packetsPerEpoch uint32, maxEpochs int) CorruptionTestResult {
	res := C.chaosseal_corruption_test(
		(C.uintptr_t)(corruptBitPos),
		(C.uint32_t)(packetsPerEpoch),
		(C.uintptr_t)(maxEpochs),
	)
	return CorruptionTestResult{
		CounterEpochsUntilDetect: int(res.counter_epochs_until_detect),
		ChaosDivergenceSteps:     int(res.chaos_divergence_steps),
		ChaosDivergenceTimeSec:   float64(res.chaos_divergence_time_sec),
		ChaosLyapunovTimescales:  float64(res.chaos_lyapunov_timescales),
		ChaosKeyDiffers:          int(res.chaos_key_differs),
	}
}
