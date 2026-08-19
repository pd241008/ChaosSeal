# core (Rust)

Protocol engine — the source of truth for all cryptographic and kinematic computations.

## Modules

| Module | Purpose |
|--------|---------|
| `fixed` | Q32.32 fixed-point arithmetic (no floats in hot path) |
| `kinematics` | Multi-pendulum ODE system + RK4 integrator |
| `lyapunov` | Benettin algorithm for Lyapunov exponent estimator (`λ1`) |
| `crypto` | HKDF → AES-256-CTR, HMAC-SHA256 commitment |
| `bee` | Subset-difference key tree, covering-set algorithm |
| `bindings` | C ABI via `cbindgen` for cgo/FFI consumption |
| `bin` | CLI (`clap`) emitting JSON |

## Dependencies

- `aes`, `ctr` — AES-256-CTR
- `hkdf` — RFC 5869 key derivation
- `hmac`, `sha2` — HMAC-SHA256
- `cbindgen` — C header generation
- `clap` — CLI

## Build & Test

```bash
cargo build --manifest-path core/Cargo.toml
cargo test --manifest-path core/Cargo.toml
```

## C ABI

The build script generates `target/.../chaosseal_core.h` exposing:
- `chaosseal_compute_lambda1(...)`
- `chaosseal_bee_ciphertext_size(n, r)`
- `chaosseal_free_string(...)`

## Tests

- AES-CTR KAT against RFC 3686 test vectors
- HMAC-SHA256 KAT against RFC 4231 test vectors
- Determinism tests (same seed → bit-exact output)
