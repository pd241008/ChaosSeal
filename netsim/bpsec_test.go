package main

import (
	"bytes"
	"testing"
)

func TestCBORPrimitives(t *testing.T) {
	// Simple values (RFC 8949 test vectors).
	cases := []struct {
		name string
		enc  []byte
		want []byte
	}{
		{"uint 0", cborUint(0), []byte{0x00}},
		{"uint 1", cborUint(1), []byte{0x01}},
		{"uint 23", cborUint(23), []byte{0x17}},
		{"uint 24", cborUint(24), []byte{0x18, 0x18}},
		{"uint 100", cborUint(100), []byte{0x18, 0x64}},
		{"uint 1000", cborUint(1000), []byte{0x19, 0x03, 0xe8}},
		{"text empty", cborText(""), []byte{0x60}},
		{"text a", cborText("a"), []byte{0x61, 0x61}},
		{"bytes empty", cborBytes(nil), []byte{0x40}},
		{"bytes 2", cborBytes([]byte{0xde, 0xad}), []byte{0x42, 0xde, 0xad}},
		{"array empty", cborArray(0), []byte{0x80}},
		{"array 3", cborArray(3), []byte{0x83}},
	}
	for _, tc := range cases {
		if !bytes.Equal(tc.enc, tc.want) {
			t.Errorf("%s: got % x, want % x", tc.name, tc.enc, tc.want)
		}
	}
}

func TestCBORDecodeRoundTrip(t *testing.T) {
	in := []interface{}{uint64(7), "ipn:1.1", []byte{1, 2, 3}, uint64(3600)}
	var encoded []byte
	encoded = append(encoded, cborArray(len(in))...)
	for _, v := range in {
		switch x := v.(type) {
		case uint64:
			encoded = append(encoded, cborUint(x)...)
		case string:
			encoded = append(encoded, cborText(x)...)
		case []byte:
			encoded = append(encoded, cborBytes(x)...)
		}
	}

	decoded, err := decodeCborArray(encoded)
	if err != nil {
		t.Fatalf("decodeCborArray: %v", err)
	}
	if len(decoded) != len(in) {
		t.Fatalf("decoded %d items, want %d", len(decoded), len(in))
	}
	if decoded[0] != uint64(7) {
		t.Errorf("item 0 = %v, want 7", decoded[0])
	}
	if decoded[1] != "ipn:1.1" {
		t.Errorf("item 1 = %v, want ipn:1.1", decoded[1])
	}
	bs, ok := decoded[2].([]byte)
	if !ok || !bytes.Equal(bs, []byte{1, 2, 3}) {
		t.Errorf("item 2 = %v, want [1 2 3]", decoded[2])
	}
	if decoded[3] != uint64(3600) {
		t.Errorf("item 3 = %v, want 3600", decoded[3])
	}
}

func TestBPv7BundleIntegrity(t *testing.T) {
	key := NewBPSecKey(42)
	iv := make([]byte, ivSizeBytes)
	for i := range iv {
		iv[i] = byte(i)
	}

	payload := []byte("LEO revocation update: revoke 8 receivers")
	payloadBlock := BuildPayloadBlock(payload)

	bcb, _, err := BuildBCB(key, iv, "key-0", payloadBlock)
	if err != nil {
		t.Fatalf("BuildBCB: %v", err)
	}
	bib, bibInfo := BuildBIB(key, "key-0", bcb)

	if len(bibInfo.MAC) != 32 {
		t.Errorf("HMAC-SHA256 length = %d, want 32", len(bibInfo.MAC))
	}

	bundle := &Bundle{
		Primary: &PrimaryBlock{Version: 7, Destination: "ipn:1.1", Source: "ipn:1.0", ReportTo: "ipn:1.0", Time: 100, Lifetime: 3600},
		Blocks:  []*CanonicalBlock{payloadBlock, bcb, bib},
	}
	wire := bundle.Encode()
	if len(wire) == 0 {
		t.Fatal("empty bundle wire format")
	}

	// Integrity check over the (encrypted) BCB data must pass.
	ok, err := VerifyBIB(key, bib, bcb)
	if err != nil || !ok {
		t.Fatalf("VerifyBIB: ok=%v err=%v", ok, err)
	}

	// Decrypt and confirm the payload round-trips.
	plain, err := DecryptPayload(key, iv, bcb, payloadBlock)
	if err != nil {
		t.Fatalf("DecryptPayload: %v", err)
	}
	if !bytes.Equal(plain, payload) {
		t.Errorf("payload mismatch: %q != %q", plain, payload)
	}
}

func TestBIBDetectsTampering(t *testing.T) {
	key := NewBPSecKey(1)
	iv := make([]byte, ivSizeBytes)

	payloadBlock := BuildPayloadBlock([]byte("original"))
	bcb, _, err := BuildBCB(key, iv, "key-0", payloadBlock)
	if err != nil {
		t.Fatal(err)
	}
	bib, _ := BuildBIB(key, "key-0", bcb)

	// Corrupt the BCB ciphertext: integrity check must now fail.
	bcb.Data[len(bcb.Data)-1] ^= 0xff
	ok, err := VerifyBIB(key, bib, bcb)
	if err != nil {
		t.Fatal(err)
	}
	if ok {
		t.Fatal("BIB accepted a tampered BCB")
	}
}

func TestWrongKeyRejected(t *testing.T) {
	key := NewBPSecKey(1)
	iv := make([]byte, ivSizeBytes)

	payloadBlock := BuildPayloadBlock([]byte("secret"))
	bcb, _, err := BuildBCB(key, iv, "key-0", payloadBlock)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := DecryptPayload(NewBPSecKey(2), iv, bcb, payloadBlock); err == nil {
		t.Fatal("decryption with wrong key succeeded, want error")
	}
}
