package main

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/hmac"
	"crypto/rand"
	"crypto/sha256"
	"encoding/binary"
	"errors"
	"fmt"
)

// ---------------------------------------------------------------------------
// Minimal CBOR encoder (RFC 8949). Only the types needed by BPv7/RFC 9172:
// unsigned integers, byte strings, text strings, and arrays.
// ---------------------------------------------------------------------------

func cborHead(major byte, arg uint64) []byte {
	h := major << 5
	switch {
	case arg < 24:
		return []byte{h | byte(arg)}
	case arg <= 0xff:
		return []byte{h | 24, byte(arg)}
	case arg <= 0xffff:
		return []byte{h | 25, byte(arg >> 8), byte(arg)}
	case arg <= 0xffffffff:
		out := []byte{h | 26}
		return append(out, uint32ToBytes(uint32(arg))...)
	default:
		out := []byte{h | 27}
		return append(out, uint64ToBytes(arg)...)
	}
}

func cborUint(v uint64) []byte { return cborHead(0, v) }

func cborText(s string) []byte {
	return append(cborHead(3, uint64(len(s))), s...)
}

func cborBytes(b []byte) []byte {
	return append(cborHead(2, uint64(len(b))), b...)
}

func cborArray(n int) []byte { return cborHead(4, uint64(n)) }

func uint32ToBytes(v uint32) []byte {
	var b [4]byte
	binary.BigEndian.PutUint32(b[:], v)
	return b[:]
}

func uint64ToBytes(v uint64) []byte {
	var b [8]byte
	binary.BigEndian.PutUint64(b[:], v)
	return b[:]
}

// ---------------------------------------------------------------------------
// BPv7 bundle structures (RFC 9171).
// ---------------------------------------------------------------------------

// PrimaryBlock is the RFC 9171 §4.2 primary block. CRC is always omitted
// (CRC type 0) to keep the wire format minimal and dependency-free.
type PrimaryBlock struct {
	Version     uint8  // always 7
	Flags       uint64 // processing control flags
	Destination string // EID, e.g. "ipn:42.1"
	Source      string // EID
	ReportTo    string // EID
	Time        uint64 // creation timestamp (seconds since DTN epoch)
	Seq         uint64 // creation timestamp sequence number
	Lifetime    uint64 // seconds
}

// CanonicalBlock is an RFC 9171 §4.3 canonical block. Data is CBOR-encoded.
type CanonicalBlock struct {
	Type uint8
	// Bits 0-5 of flags hold the block type; bit 7 = must be replicated.
	Flags uint8
	Data  []byte
}

// Payload block type code per RFC 9171 §4.5.
const (
	BlockTypePayload = 1
	BlockTypeBCB     = 10
	BlockTypeBIB     = 11
)

// Encode serializes the primary block to its on-wire form.
func (p *PrimaryBlock) Encode() []byte {
	out := []byte{p.Version, 0x01} // version byte + block processing control flags
	arr := append(cborArray(8),
		cborUint(uint64(p.Version))...,
	)
	arr = append(arr, cborUint(p.Flags)...)
	arr = append(arr, cborUint(0)...) // CRC type: none
	arr = append(arr, cborText(p.Destination)...)
	arr = append(arr, cborText(p.Source)...)
	arr = append(arr, cborText(p.ReportTo)...)
	arr = append(arr, cborArray(2)...)
	arr = append(arr, cborUint(p.Time)...)
	arr = append(arr, cborUint(p.Seq)...)
	arr = append(arr, cborUint(p.Lifetime)...)
	return append(out, arr...)
}

// Encode serializes a canonical block to its on-wire form.
func (b *CanonicalBlock) Encode() []byte {
	out := []byte{b.Type, b.Flags}
	return append(out, b.Data...)
}

// Bundle is a complete BPv7 bundle: primary block followed by canonical blocks.
type Bundle struct {
	Primary *PrimaryBlock
	Blocks  []*CanonicalBlock
}

// Encode serializes the full bundle.
func (b *Bundle) Encode() []byte {
	out := b.Primary.Encode()
	for _, blk := range b.Blocks {
		out = append(out, blk.Encode()...)
	}
	return out
}

// ---------------------------------------------------------------------------
// BPSec canonical blocks (RFC 9172).
// ---------------------------------------------------------------------------

const (
	// Security context identifiers (RFC 9172 conventions).
	ctxBIBHMACSHA256 = "bib-hmac-sha256"
	ctxBCBAES256GCM  = "bcb-aes256-gcm"
	keySizeBits      = 256
	ivSizeBytes      = 12 // GCM standard nonce
	gcmTagBytes      = 16
)

// BuildPayloadBlock encodes a payload block carrying arbitrary bytes.
func BuildPayloadBlock(payload []byte) *CanonicalBlock {
	data := append(cborArray(2),
		cborUint(0)..., // wrapping flag 0 = not wrapped
	)
	data = append(data, cborBytes(payload)...)
	return &CanonicalBlock{Type: BlockTypePayload, Flags: 0, Data: data}
}

// BIBBlock carries an HMAC-SHA256 integrity value over covered blocks.
type BIBBlock struct {
	KeyID string
	MAC   []byte
}

// BuildBIB computes and encodes an RFC 9172 BIB (HMAC-SHA256) over the given
// covered blocks' data (all bytes after each block's two header bytes).
func BuildBIB(key []byte, keyID string, covered ...*CanonicalBlock) (*CanonicalBlock, *BIBBlock) {
	macInput := make([]byte, 0)
	for _, blk := range covered {
		macInput = append(macInput, blk.Data...)
	}
	mac := hmacSHA256(key, macInput)

	data := append(cborArray(4),
		cborText(ctxBIBHMACSHA256)...,
	)
	data = append(data, cborText(keyID)...)
	data = append(data, cborArray(0)...) // MAC parameters: none
	data = append(data, cborBytes(mac)...)
	return &CanonicalBlock{Type: BlockTypeBIB, Flags: 0, Data: data}, &BIBBlock{KeyID: keyID, MAC: mac}
}

// BCBBlock carries the AES-256-GCM confidentiality parameters.
type BCBBlock struct {
	KeyID      string
	IV         []byte
	Ciphertext []byte
}

// BuildBCB encrypts the given payload block plaintext with AES-256-GCM and
// produces an RFC 9172 BCB block. The payload block is replaced with its
// wrapped (encrypted) form.
func BuildBCB(key, iv []byte, keyID string, payload *CanonicalBlock) (*CanonicalBlock, *BCBBlock, error) {
	if len(key) != keySizeBits/8 {
		return nil, nil, fmt.Errorf("BCB requires a %d-bit key, got %d bytes", keySizeBits, len(key))
	}
	if len(iv) != ivSizeBytes {
		return nil, nil, fmt.Errorf("BCB requires a %d-byte IV, got %d bytes", ivSizeBytes, len(iv))
	}

	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, nil, err
	}
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, nil, err
	}
	ciphertext := gcm.Seal(nil, iv, payload.Data, nil)

	data := append(cborArray(4),
		cborText(ctxBCBAES256GCM)...,
	)
	data = append(data, cborText(keyID)...)
	data = append(data, cborBytes(iv)...)
	data = append(data, cborBytes(ciphertext)...)

	bcb := &CanonicalBlock{Type: BlockTypeBCB, Flags: 0, Data: data}

	// Replace the payload block with its wrapped (encrypted) form per
	// RFC 9172 §4.1.2: wrapping flag 1 + ciphertext.
	wrapped := append(cborArray(2),
		cborUint(1)...,
	)
	wrapped = append(wrapped, cborBytes(ciphertext)...)
	payload.Type = BlockTypePayload
	payload.Data = wrapped

	return bcb, &BCBBlock{KeyID: keyID, IV: iv, Ciphertext: ciphertext}, nil
}

// VerifyBIB recomputes the HMAC over the covered blocks and compares it to
// the value carried in the BIB block data.
func VerifyBIB(key []byte, bib *CanonicalBlock, covered ...*CanonicalBlock) (bool, error) {
	if bib.Type != BlockTypeBIB {
		return false, fmt.Errorf("not a BIB block (type %d)", bib.Type)
	}
	items, err := decodeCborArray(bib.Data)
	if err != nil {
		return false, err
	}
	if len(items) != 4 {
		return false, fmt.Errorf("BIB data has %d items, want 4", len(items))
	}
	ctx, ok := items[0].(string)
	if !ok || ctx != ctxBIBHMACSHA256 {
		return false, fmt.Errorf("unexpected BIB security context %v", items[0])
	}
	mac, ok := items[3].([]byte)
	if !ok {
		return false, errors.New("BIB MAC is not a byte string")
	}
	macInput := make([]byte, 0)
	for _, blk := range covered {
		macInput = append(macInput, blk.Data...)
	}
	expected := hmacSHA256(key, macInput)
	return hmac.Equal(mac, expected), nil
}

// DecryptPayload unwraps a BCB-protected payload block and returns the
// plaintext bytes.
func DecryptPayload(key, iv []byte, bcb *CanonicalBlock, payload *CanonicalBlock) ([]byte, error) {
	if bcb.Type != BlockTypeBCB {
		return nil, fmt.Errorf("not a BCB block (type %d)", bcb.Type)
	}
	if len(key) != keySizeBits/8 {
		return nil, fmt.Errorf("BCB requires a %d-bit key", keySizeBits)
	}
	if len(iv) != ivSizeBytes {
		return nil, fmt.Errorf("BCB requires a %d-byte IV", ivSizeBytes)
	}
	items, err := decodeCborArray(bcb.Data)
	if err != nil {
		return nil, err
	}
	if len(items) != 4 {
		return nil, fmt.Errorf("BCB data has %d items, want 4", len(items))
	}
	ciphertext, ok := items[3].([]byte)
	if !ok {
		return nil, errors.New("BCB ciphertext is not a byte string")
	}

	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}
	plain, err := gcm.Open(nil, iv, ciphertext, nil)
	if err != nil {
		return nil, err
	}

	// Unwrap the payload block.
	payloadItems, err := decodeCborArray(plain)
	if err != nil {
		return nil, err
	}
	if len(payloadItems) < 2 {
		return nil, errors.New("wrapped payload has too few elements")
	}
	raw, ok := payloadItems[1].([]byte)
	if !ok {
		return nil, errors.New("wrapped payload is not a byte string")
	}
	return raw, nil
}

// NewBPSecKey derives a deterministic 256-bit key from a seed, keeping runs
// reproducible.
func NewBPSecKey(seed uint64) []byte {
	var in [8]byte
	binary.BigEndian.PutUint64(in[:], seed)
	key := sha256.Sum256(in[:])
	return key[:]
}

func hmacSHA256(key, msg []byte) []byte {
	h := hmac.New(sha256.New, key)
	h.Write(msg)
	return h.Sum(nil)
}

// ---------------------------------------------------------------------------
// Minimal CBOR decoder for the subset emitted by the encoder above.
// ---------------------------------------------------------------------------

// decodeCborArray decodes a definite-length CBOR array whose elements are
// unsigned integers, byte strings, text strings, or nested arrays.
func decodeCborArray(data []byte) ([]interface{}, error) {
	items, _, err := decodeCborArrayWithLen(data)
	return items, err
}

// decodeCborArrayWithLen decodes an array and reports how many bytes it
// occupied on the wire.
func decodeCborArrayWithLen(data []byte) ([]interface{}, int, error) {
	if len(data) < 1 {
		return nil, 0, errors.New("empty CBOR data")
	}
	head := data[0]
	major := head >> 5
	if major != 4 {
		return nil, 0, fmt.Errorf("expected CBOR array (major 4), got major %d", major)
	}
	n, consumed, err := cborArg(data)
	if err != nil {
		return nil, 0, err
	}
	rest := data[consumed:]
	total := consumed
	out := make([]interface{}, 0, n)
	for i := uint64(0); i < n; i++ {
		if len(rest) < 1 {
			return nil, 0, errors.New("truncated CBOR array")
		}
		val, used, err := decodeCborValue(rest)
		if err != nil {
			return nil, 0, err
		}
		out = append(out, val)
		total += used
		rest = rest[used:]
	}
	return out, total, nil
}

// decodeCborValue decodes a single CBOR value (uint, byte string, text
// string, or nested array) and returns it plus the number of bytes consumed.
func decodeCborValue(data []byte) (interface{}, int, error) {
	if len(data) < 1 {
		return nil, 0, errors.New("empty CBOR data")
	}
	major := data[0] >> 5
	arg, used, err := cborArg(data)
	if err != nil {
		return nil, 0, err
	}
	switch major {
	case 0:
		return arg, used, nil
	case 2, 3:
		b, n, err := cborTake(data, used, arg)
		if err != nil {
			return nil, 0, err
		}
		if major == 3 {
			return string(b), used + n, nil
		}
		return b, used + n, nil
	case 4:
		sub, used, err := decodeCborArrayWithLen(data)
		return sub, used, err
	default:
		return nil, 0, fmt.Errorf("unsupported CBOR major type %d in decoder", major)
	}
}

func cborArg(data []byte) (uint64, int, error) {
	if len(data) < 1 {
		return 0, 0, errors.New("empty CBOR data")
	}
	ai := data[0] & 0x1f
	consumed := 1
	switch {
	case ai < 24:
		return uint64(ai), consumed, nil
	case ai == 24:
		if len(data) < 2 {
			return 0, 0, errors.New("truncated CBOR uint8")
		}
		return uint64(data[1]), consumed + 1, nil
	case ai == 25:
		if len(data) < 3 {
			return 0, 0, errors.New("truncated CBOR uint16")
		}
		return uint64(binary.BigEndian.Uint16(data[1:3])), consumed + 2, nil
	case ai == 26:
		if len(data) < 5 {
			return 0, 0, errors.New("truncated CBOR uint32")
		}
		return uint64(binary.BigEndian.Uint32(data[1:5])), consumed + 4, nil
	case ai == 27:
		if len(data) < 9 {
			return 0, 0, errors.New("truncated CBOR uint64")
		}
		return binary.BigEndian.Uint64(data[1:9]), consumed + 8, nil
	default:
		return 0, 0, errors.New("reserved CBOR additional info")
	}
}

func cborTake(data []byte, afterHead int, n uint64) ([]byte, int, error) {
	start := uint64(afterHead)
	if start+n > uint64(len(data)) {
		return nil, 0, errors.New("truncated CBOR byte/text string")
	}
	return data[start : start+n], int(n), nil
}

// NewRandomIV returns a fresh 12-byte GCM nonce from crypto/rand.
func NewRandomIV() ([]byte, error) {
	iv := make([]byte, ivSizeBytes)
	if _, err := rand.Read(iv); err != nil {
		return nil, err
	}
	return iv, nil
}
