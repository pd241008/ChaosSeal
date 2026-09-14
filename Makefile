# Reproducibility targets for the ChaosSeal (CEP) artifact.
#
# Default toolchain: cargo (Rust >= 1.75), go (>= 1.22), python3 (>= 3.10).
# GPU is not used anywhere; the Q32.32 fixed-point core is single-core CPU
# by design (cross-platform bit-exactness is the point).
#
# Runtimes quoted were measured on the development machine (10-core CPU):
# a single sim run is ~0.3 s; the full driver including all tiers is minutes;
# the T=4000 s Lyapunov spectrum horizon is minutes (single core).
#
# Sim re-runs never write into the canonical archive results_v3/: fresh
# outputs go to results_fresh/ (or an explicit directory you pass). The two
# pure-analysis targets (reproduce-commit-loss, reproduce-bootstrap)
# regenerate their own derived CSVs/figures in results_v3/ from the archived
# run data, by design.

SHELL := /bin/bash
PY ?= python3

RUSTCLI := $(CURDIR)/core_v2/target/release/chaosseal
CLIV2   := $(CURDIR)/core_v2/target/release/cli_v2
SIM     := $(CURDIR)/netsim_v2/chaoseal-sim

.PHONY: help build test verify smoke \
        reproduce-rsweep reproduce-loss-sweep reproduce-size-sweep \
        reproduce-commit-sweep reproduce-corruption reproduce-membership \
        reproduce-spectrum reproduce-lambda-min reproduce-robustness \
        reproduce-commit-loss reproduce-bootstrap reproduce-analysis \
        firmware-bench firmware-hw firmware-flash reproduce-all clean

help:
	@echo "Targets:"
	@echo "  make build                  build Rust core + Go simulator"
	@echo "  make test                   Rust tests (9 lib + 14 KAT) and Go tests"
	@echo "  make verify                 independent verification gates (Benettin replicator, metastability)"
	@echo "  make smoke                  build + test + verify + one sim run + aggregation"
	@echo "  make reproduce-rsweep       Tier 1a: chaosseal vs counter vs BPSec, 5 seeds x 10 R"
	@echo "  make reproduce-loss-sweep   Tier 2a: fixed packet-loss sweep"
	@echo "  make reproduce-size-sweep   Tier 2b: payload-size sweep"
	@echo "  make reproduce-commit-sweep Tier 2c: commitment interval N sweep"
	@echo "  make reproduce-corruption   single-bit corruption detection test"
	@echo "  make reproduce-membership   dynamic-membership join scenario"
	@echo "  make reproduce-spectrum     full Lyapunov spectrum + convergence horizons"
	@echo "  make reproduce-lambda-min   lambda_min attractor-trial series"
	@echo "  make reproduce-robustness   pendulum parameter robustness sweep"
	@echo "  make reproduce-commit-loss  commitment interval x LEO loss profiles"
	@echo "  make reproduce-bootstrap    bootstrap CI on the crossover point"
	@echo "  make reproduce-analysis     regenerate stats CSVs + figures from the archive"
	@echo "  make reproduce-all          everything except the long spectrum horizons"
	@echo "  make firmware-bench         build no_std Cortex-M4 bench + run under QEMU (skips run if QEMU absent)"
	@echo "  make firmware-hw            build the hardware image (UART console) + bench.bin"
	@echo "  make firmware-flash         flash bench.bin to the STM32F4 via st-flash (Midas-style)"
	@echo "  make clean                  remove fresh outputs"

# ---------------------------------------------------------------------------
build:
	cd core_v2 && cargo build --release
	cd netsim_v2 && CGO_ENABLED=1 go build -o chaoseal-sim .

test:
	cd core_v2 && cargo test --release
	cd netsim_v2 && go vet ./... && go test ./...

# --- Independent verification gates ----------------------------------------
verify:
	$(PY) scripts/validate_benettin.py
	$(PY) scripts/verify_metastability.py

# --- One end-to-end run + archive aggregation -------------------------------
smoke: build test verify
	$(SIM) --run-id smoke --seed 1 --bee-r 8 \
	  --baselines chaosseal,counter,bpsec \
	  --results-dir results_fresh --rust-cli "$(RUSTCLI)"
	$(PY) analysis/v3_analysis.py

# --- Tier 1a: R-sweep (C1, C6 data source) ----------------------------------
reproduce-rsweep: build
	@set -e; for seed in 1 2 3 4 5; do for r in 1 2 4 8 16 32 64 128 256 512; do \
	  printf -v rpad "%04d" $$r; \
	  $(SIM) --run-id "v3-rsweep-seed$$seed-r$$rpad" --seed $$seed --bee-r $$r \
	    --baselines chaosseal,counter,bpsec \
	    --results-dir results_fresh --rust-cli "$(RUSTCLI)"; \
	done; done
	$(PY) analysis/v3_analysis.py

# --- Tier 2a: packet-loss sweep (C8) ----------------------------------------
reproduce-loss-sweep: build
	@set -e; for loss in 0.0 0.01 0.03 0.05 0.10; do \
	  lbl=$$($(PY) -c "print(f'{$${loss}:.2f}'.replace('.','p'))"); \
	  $(SIM) --run-id "v3-loss-sweep-$$lbl" --seed 1 --bee-r 8 \
	    --baselines chaosseal,counter,bpsec --loss-rate $$loss \
	    --results-dir results_fresh --rust-cli "$(RUSTCLI)"; \
	done

# --- Tier 2b: packet-size sweep (C9) ----------------------------------------
reproduce-size-sweep: build
	@set -e; for size in 128 256 512 1024 2048 4096; do \
	  $(SIM) --run-id "v3-size-sweep-$$size" --seed 1 --bee-r 8 \
	    --baselines chaosseal,counter,bpsec --payload-bytes $$size \
	    --results-dir results_fresh --rust-cli "$(RUSTCLI)"; \
	done

# --- Tier 2c: commitment interval N sweep (C5 input) ------------------------
reproduce-commit-sweep: build
	@set -e; for n in 1 4 16 64 256 1024; do \
	  $(SIM) --run-id "v3-commit-sweep-n$$n" --seed 1 --bee-r 8 \
	    --baselines chaosseal --commit-interval $$n \
	    --results-dir results_fresh --rust-cli "$(RUSTCLI)"; \
	done

# --- Single-bit corruption detection (C2) -----------------------------------
reproduce-corruption: build
	$(SIM) --run-id corruption-test --corruption-test \
	  --results-dir results_fresh --rust-cli "$(RUSTCLI)"

# --- Dynamic membership / join protocol (C7) --------------------------------
reproduce-membership: build
	$(SIM) --run-id membership-join-n1024-r8 --membership-test --membership-joins 8 \
	  --results-dir results_fresh --rust-cli "$(RUSTCLI)"
	$(CLIV2) join-protocol --n 1024 --r 8 --joins 20 \
	  --rebuild-interval-epochs 6.0 --epoch-duration-s 1200

# --- Lyapunov spectrum + convergence horizons (C4) --------------------------
# Long: the T=4000 s horizon is minutes on a single core. The shipped archive
# lives in results_v3/convergence/; this target re-generates it.
reproduce-spectrum: build
	$(PY) scripts/verify_spectrum_convergence.py

# --- lambda_min attractor-trial series (C3, C10) ----------------------------
reproduce-lambda-min: build
	$(PY) scripts/regen_lambda_min_series.py

# --- Pendulum parameter robustness sweep (C3) -------------------------------
reproduce-robustness: build
	SWEEP_WORKERS=6 $(PY) scripts/regen_robustness.py

# --- Commitment interval x LEO loss profiles (C5) ---------------------------
reproduce-commit-loss:
	$(PY) analysis/commit_loss_sweep.py

# --- Bootstrap CI on the crossover point (C6) -------------------------------
reproduce-bootstrap:
	$(PY) analysis/bootstrap_crossover.py

# --- Archive aggregation: stats CSVs + figures (all claims) -----------------
reproduce-analysis:
	$(PY) analysis/v3_analysis.py
	$(PY) analysis/v4_generalization.py

# --- Cortex-M4 firmware benchmark (QEMU-first) ------------------------------
# QEMU build uses the semihosting console (--features qemu; semihosting
# BKPTs would hard-fault real hardware without a debugger). The default
# build is the hardware image: bare-metal USART2 console + LEDs + 168 MHz
# PLL init, Midas-artifact style. See firmware/stm32f4-bench/README.md.
firmware-bench:
	cd firmware/stm32f4-bench && cargo build --release --features qemu --no-default-features
	@if command -v qemu-system-arm >/dev/null 2>&1; then \
	  cd firmware/stm32f4-bench && qemu-system-arm -machine netduinoplus2 -nographic \
	    -semihosting-config enable=on,target=native -icount shift=0 \
	    -kernel target/thumbv7em-none-eabihf/release/stm32f4-bench; \
	else \
	  echo "qemu-system-arm not found: firmware built (flash-ready), emulated run skipped"; \
	fi

# Hardware image + Midas-style flashing (ST-LINK attached to the host; on
# WSL attach it via usbipd first). Console: USART2 @ 115200 8N1 on PA2/PA3.
firmware-hw:
	cd firmware/stm32f4-bench && cargo build --release
	arm-none-eabi-objcopy -O binary \
	  firmware/stm32f4-bench/target/thumbv7em-none-eabihf/release/stm32f4-bench \
	  firmware/stm32f4-bench/bench.bin
	arm-none-eabi-size firmware/stm32f4-bench/target/thumbv7em-none-eabihf/release/stm32f4-bench

firmware-flash: firmware-hw
	st-flash write firmware/stm32f4-bench/bench.bin 0x08000000
	@echo "Console: USART2 @ 115200 8N1 on PA2 (TX) — e.g. 'screen /dev/ttyUSB0 115200'"

# --- Everything except the long spectrum horizons ---------------------------
reproduce-all: reproduce-rsweep reproduce-loss-sweep reproduce-size-sweep \
	reproduce-commit-sweep reproduce-corruption reproduce-membership \
	reproduce-commit-loss reproduce-bootstrap reproduce-analysis

clean:
	rm -rf results_fresh
	find . -name __pycache__ -type d -exec rm -rf {} + 2>/dev/null || true
