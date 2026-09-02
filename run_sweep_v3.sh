#!/bin/bash
set -e

# Build Rust core and Go simulator once.
echo "=== Building Rust core (core_v2) ==="
(cd core_v2 && cargo build --release)

echo "=== Building Go simulator ==="
(cd netsim_v2 && go clean -cache && CGO_ENABLED=1 go build -o chaoseal-sim .)

SIM="./netsim_v2/chaoseal-sim"
RUSTCLI="$(pwd)/core_v2/target/release/chaosseal"

# Directory layout
BASE="${1:-results_v3}"
mkdir -p "$BASE"

# ---------------------------------------------------------------------------
# Tier 1a: Goodput sweep of pendulum (chaosseal) vs counter baseline across R.
# N=1024, epoch=1200s, 5 seeds, R in {1,2,4,8,16,32,64,128,256,512}.
# ---------------------------------------------------------------------------
echo "=== Tier 1a: R-sweep: chaosseal vs counter vs bpsec (5 seeds) ==="
for seed in 1 2 3 4 5; do
    for r in 1 2 4 8 16 32 64 128 256 512; do
        printf -v rpad "%04d" $r
        $SIM --run-id "v3-rsweep-seed${seed}-r${rpad}" \
             --seed $seed --bee-r $r --baselines "chaosseal,counter,bpsec" \
             --results-dir "$BASE" --rust-cli "$RUSTCLI"
    done
done

# ---------------------------------------------------------------------------
# Tier 2a: Packet-loss sweep (fixed loss, R=8) — chaosseal vs counter vs bpsec.
# ---------------------------------------------------------------------------
echo "=== Tier 2a: Packet-loss sweep (fixed loss rate, R=8) ==="
for loss in 0.0 0.01 0.03 0.05 0.10; do
    lbl=$(python3 -c "print(f'{${loss}:.2f}'.replace('.','p'))")
    $SIM --run-id "v3-loss-sweep-${lbl}" \
         --seed 1 --bee-r 8 --baselines "chaosseal,counter,bpsec" \
         --loss-rate $loss --results-dir "$BASE" --rust-cli "$RUSTCLI"
done

# ---------------------------------------------------------------------------
# Tier 2b: Packet-size sweep — chaosseal vs counter vs bpsec.
# ---------------------------------------------------------------------------
echo "=== Tier 2b: Packet-size sweep (R=8) ==="
for size in 128 256 512 1024 2048 4096; do
    $SIM --run-id "v3-size-sweep-${size}" \
         --seed 1 --bee-r 8 --baselines "chaosseal,counter,bpsec" \
         --payload-bytes $size --results-dir "$BASE" --rust-cli "$RUSTCLI"
done

# ---------------------------------------------------------------------------
# Tier 2c: Commitment interval N sensitivity (chaosseal only).
# This is a modeling exercise: the HMAC verifies every N packets. Smaller N =
# faster corruption detection but higher overhead; larger N = converse.
# We emulate by varying DownlinkBps-scale per-packet sync cost.
# ---------------------------------------------------------------------------
echo "=== Tier 2c: Commitment interval N sensitivity (R=8) ==="
# N in packets: 1, 4, 16, 64, 256, 1024. Overhead: each verification costs an
# extra HMAC-processed 32-byte tag per verified packet (amortized 32/N bytes).
for n in 1 4 16 64 256 1024; do
    $SIM --run-id "v3-commit-sweep-n${n}" \
         --seed 1 --bee-r 8 --baselines "chaosseal" \
         --commit-interval $n --results-dir "$BASE" --rust-cli "$RUSTCLI"
done

echo "=== All sweeps complete. Results in $BASE ==="
