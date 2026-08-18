package client

/*
#cgo LDFLAGS: ${SRCDIR}/../../../target/release/libchaosseal_core.a -lpthread -ldl -lm
#include <stdint.h>
#include <stdlib.h>

typedef struct {
    uintptr_t ciphertext_len;
    uintptr_t hmac_len;
} CEpochCryptoResult;

CEpochCryptoResult chaosseal_epoch_crypto(const uint8_t* payload, uintptr_t payload_len);
*/
import "C"
import "unsafe"

// EpochCryptoResult represents the sizes returned by the simulated epoch crypto.
type EpochCryptoResult struct {
	CiphertextLen int
	HmacLen       int
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
		CiphertextLen: int(res.ciphertext_len),
		HmacLen:       int(res.hmac_len),
	}
}
