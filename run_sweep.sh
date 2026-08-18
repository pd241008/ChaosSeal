#!/bin/bash
set -e

echo "Cleaning old results..."
rm -rf results/*.json

echo "Running R sweeps across 5 seeds..."
cd netsim
for seed in 1 2 3 4 5; do
    for r in 1 2 4 8 16 32 64 128 256 512; do
        printf -v rpad "%04d" $r
        echo "Seed=$seed, R=$r..."
        go run . --run-id "sweep-seed${seed}-r${rpad}" --seed $seed --bee-r $r --results-dir results --rust-cli "../target/release/chaosseal"
    done
done
cd ..

echo "Generating CSVs..."
python3 export_csv.py
python3 analysis/repeat_sweep.py raw_output.csv > results/sweep_stats.csv
echo "Done."
