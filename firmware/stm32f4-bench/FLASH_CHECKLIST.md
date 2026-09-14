# Flash & Capture Checklist — STM32F4 Discovery

Everything to do at the board, in order. Total time: ~5 minutes.
No debugger required for the run itself, and **no serial adapter is needed
for capture** — the firmware mirrors its console into SRAM and OpenOCD dumps
it (`make firmware-capture`, step 4b). The UART console (step 4a) is only
for live watching.

## 0. What you need

- STM32F4 Discovery (STM32F407VG) + mini-USB cable (ST-LINK side, CN1)
- *Optional* (live console only): a 3.3 V USB-serial adapter —
  PA2 (USART2 TX) → adapter RX, GND → GND (PA3/RX unused; TX-only console)
- Host tools: `st-flash` (stlink-tools), `openocd`, `arm-none-eabi-objcopy`,
  and only for the live console a serial terminal (`screen`, `minicom`, PuTTY)

## 1. Build the hardware image

```bash
make firmware-hw     # repo root; produces firmware/stm32f4-bench/bench.bin
```

Size report should show roughly: text ~110 KB, bss ~49 KB
(1 MB flash / 192 KB SRAM budget → ~11% / ~26%).

## 2. Connect ST-LINK

- **Linux host**: just plug in the mini-USB (CN1). Check with
  `lsusb` → `STMicroelectronics ST-LINK/V2`.
- **WSL**: attach first:
  ```powershell
  usbipd list                       # find the ST-LINK VID:PID
  usbipd bind --hardware-id <VID:PID>
  usbipd attach --wsl --hardware-id <VID:PID>
  ```

## 3. Flash

```bash
make firmware-flash   # = st-flash --connect-under-reset write bench.bin 0x08000000
```

Expected tail: `Flash written and verified! jolly good!` (or similar).
If you get `Can not connect to target`, the `--connect-under-reset` flag
(hardware reset line) is the fix — already in the Makefile target.

## 4a. Live console (optional — needs the USB-serial adapter)

```bash
screen /dev/ttyUSB0 115200    # or: minicom -D /dev/ttyUSB0 -b 115200
```
(Exit screen: Ctrl-A then K.)

## 4b. Serial-free capture (no adapter — OpenOCD SRAM dump)

```bash
make firmware-capture
```

This flashes, resets/runs the board, polls the SRAM2 done flag, halts,
dumps the bench log, and parses it to `bench_hw.json`. The parser fails
hard if the SysTick/DWT probes disagree >5% or any gate fails. Expected
tail:

```
bench-log captured: status=1 len=869 -> /tmp/chaosseal_bench_dump.bin
status: 1 (all gates passed)
  ... [ok]/[bench] lines ...
probe cross-check: systick=1200006 dwt=1200011 rel=0.00% OK
json written: firmware/stm32f4-bench/bench_hw.json
```

## 5. Reset the board (black button, NRST) and watch

**LEDs:**
| LED | Meaning |
|---|---|
| Green (PD12) ON | benchmark running |
| Green OFF | done successfully (`[done]` line printed) |
| Red (PD14) steady | failure — a gate failed or a panic occurred |

**Expected output** (order matters; values from the archived hardware
capture — your board should land within a few % on the same model):

```
[probe] ... SysTick + DWT dual-clock deltas   (two bench lines)
[chaosseal-bench] Cortex-M4F Q32.32 protocol benchmark
[chaosseal-bench] core_v2 @ 29be3b5; SysTick timing (HW: cycles, QEMU: insns)
[ok] HMAC-SHA256 RFC 4231 case 1
[ok] AES-256-GCM 1024B roundtrip + tag length
[bench] epoch_rk4_step <N> tck/step           <- REAL CYCLES now, not insns
[bench] epoch_rk4_epoch <N> tck/epoch
[bench] lyapunov_benettin <N> tck/step
[bench] lambda1_sanity_millinats <N> mnats/s @2000steps
[bench] hkdf_packet_key <N> tck/op
[bench] aes256gcm_enc_1024B <N> tck/op
[bench] hmac_commit_1024B <N> tck/op
[bench] packet_total <N> tck/pkt
[ok] HMAC commitment verify roundtrip
[bench] epoch_maintenance_ticks <N> tck/epoch
[bench] per_packet_ticks <N> tck/pkt
[bench] epoch_over_packet_budget <N> pkts
[done] all gates passed
```

On hardware, SysTick ticks are **true CPU cycles** (168 MHz after the PLL
init) and the `systick_probe` / `dwt_probe` lines should be within a few
percent of each other — that is the cross-check (measured: 0.0004% apart).
Measured bench numbers run 7.3–8.0× above the QEMU instruction counts
(flash wait states, multi-cycle instructions); the Lyapunov step is the
outlier at 1.14× (FPU + hardware divide).

## 6. Report back

With `make firmware-capture` this is automatic: the parsed log lands in
`firmware/stm32f4-bench/bench_hw.json` and is folded into
`bench_results.json`. For a live-console run, copy the whole serial log
(including the two probe lines).

## Troubleshooting

| Symptom | Likely cause / fix |
|---|---|
| `st-flash` finds no device | WSL: usbipd not attached; Linux: check `lsusb`, try `sudo` |
| `Can not connect to target` | use `--connect-under-reset` (in the Makefile); check the board is powered via CN1 |
| No serial output | TX/RX swapped? baud set to 115200 8N1? adapter is 3.3 V (5 V adapters can damage PA2) |
| Garbled serial | wrong baud (must be 115200) or bad GND |
| Nothing at all, no LEDs | image not flashed (redo step 3), or boot pins disturbed |
| Red LED immediately | a gate failed — capture the serial line above the red LED; run `make firmware-failtest` in QEMU to see what a failure looks like |
| Numbers wildly off (~10×) | console works but PLL init failed — check HSE oscillator (should never happen; probes would show it) |

## Negative test (optional, QEMU-side)

```bash
make firmware-failtest
```
Prints `[fail] HMAC-SHA256 RFC 4231 KAT mismatch` and **never** `[done]` —
this is what a broken gate looks like. Rebuild with `make firmware-bench`
to restore the passing binary.
