#!/bin/bash
set -e

echo "Building v2 Rust core..."
cd core_v2
cargo build --release
cd ..

echo "Cleaning old results..."
rm -rf results_v2
mkdir -p results_v2

echo "Running R sweeps across 5 seeds with V2 engine..."
cd netsim_v2
for seed in 1 2 3 4 5; do
    for r in 1 2 4 8 16 32 64 128 256 512; do
        printf -v rpad "%04d" $r
        echo "Seed=$seed, R=$r..."
        go run . --run-id "sweep-v2-seed${seed}-r${rpad}" --seed $seed --bee-r $r --results-dir results_v2 --rust-cli "../core_v2/target/release/chaosseal"
    done
done
cd ..

echo "Generating CSVs for v2..."
PYTHONPATH=. python3 scripts/export_csv_v2.py || echo "export_csv failed"
python3 analysis/repeat_sweep.py raw_output_v2.csv > results_v2/sweep_stats.csv || echo "analysis failed"
echo "Done."
