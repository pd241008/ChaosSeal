#!/usr/bin/env python3
"""Parse the SRAM bench-log dump captured from the STM32F4 Discovery.

The hardware firmware (feature ``hw``) mirrors its console lines into a
reserved SRAM2 region at 0x2001C000 and publishes a header once done:

    +0x00  u32  magic  "CSBL" = 0x4353424C (little-endian)
    +0x04  u32  status 1 = all gates passed, 2 = a gate failed
    +0x08  u32  length log bytes at +0x10
    +0x10  u8[length]  newline-separated console-mirror lines

Input: raw dump of the region (openocd dump_image via
firmware/stm32f4-bench/openocd-capture.cfg), default
/tmp/chaosseal_bench_dump.bin.

Behavior: prints every log line, cross-checks the SysTick vs DWT probes of
the same 100k-iteration loop (must agree within 5% — both run on the CPU
clock after PLL init, so disagreement means clock bring-up is suspect), and
optionally writes a JSON summary.

    python3 scripts/parse_bench_dump.py [--dump PATH] [--json OUT.json]
"""

import argparse
import json
import struct
import sys

MAGIC = 0x4353424C  # "CSBL" little-endian
HEADER = 0x10
MAX_LOG = 4096

STATUS = {1: "all gates passed", 2: "a gate failed"}


def parse(dump_path: str) -> dict:
    with open(dump_path, "rb") as f:
        raw = f.read()
    if len(raw) < HEADER:
        sys.exit(f"dump too small ({len(raw)} B): {dump_path}")
    magic, status, length = struct.unpack_from("<III", raw, 0)
    if magic != MAGIC:
        sys.exit(
            f"bad magic 0x{magic:08X} (expected 0x{MAGIC:08X}) — "
            "not a ChaosSeal bench log"
        )
    if status not in STATUS:
        sys.exit(f"bad status word {status}")
    length = min(length, MAX_LOG)
    if len(raw) < HEADER + length:
        sys.exit(
            f"dump truncated: header says {length} log bytes, "
            f"file has {len(raw) - HEADER}"
        )
    text = raw[HEADER : HEADER + length].decode("ascii", "replace")
    lines = [ln for ln in text.split("\n") if ln.strip()]
    return {"status": status, "status_str": STATUS[status], "lines": lines}


def summarize(lines: list) -> dict:
    bench = {}
    for ln in lines:
        if ln.startswith("[bench] "):
            parts = ln.split()
            if len(parts) >= 4:
                try:
                    bench[parts[1]] = int(parts[2])
                except ValueError:
                    pass
    return bench


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--dump", default="/tmp/chaosseal_bench_dump.bin")
    ap.add_argument(
        "--json", dest="json_out", default=None,
        help="also write parsed values to this JSON file",
    )
    args = ap.parse_args()

    log = parse(args.dump)
    print(f"status: {log['status']} ({log['status_str']})")
    for ln in log["lines"]:
        print(f"  {ln}")

    bench = summarize(log["lines"])
    if not bench:
        sys.exit("no [bench] lines found in log")

    checks = {}
    st, dwt = bench.get("systick_probe"), bench.get("dwt_probe")
    if st and dwt:
        rel = abs(st - dwt) / max(st, dwt)
        ok = rel < 0.05
        checks["probe_rel_diff"] = round(rel, 4)
        checks["probe_agree_5pct"] = ok
        print(
            f"probe cross-check: systick={st} dwt={dwt} rel={rel:.2%} "
            f"{'OK' if ok else 'MISMATCH (>5%)'}"
        )
        if not ok:
            sys.exit("SysTick/DWT probes disagree — clock init suspect")

    if args.json_out:
        out = {
            "platform": "STM32F407VG Discovery (hardware)",
            "clock_hz": 168_000_000,
            "clock_source": "HSE 8 MHz PLL (M=8 N=336 P=2 Q=7), Midas constants",
            "timer": "SysTick @ CPU clock (ticks = cycles) + DWT CYCCNT probe",
            "dump": args.dump,
            "status": log["status"],
            "gates_passed": log["status"] == 1,
            "bench": bench,
            "checks": checks,
        }
        with open(args.json_out, "w") as f:
            json.dump(out, f, indent=2)
        print(f"json written: {args.json_out}")

    if log["status"] != 1:
        sys.exit("gate FAILED on hardware (status=2)")


if __name__ == "__main__":
    main()
