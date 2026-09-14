MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 1024K
  RAM   : ORIGIN = 0x20000000, LENGTH = 112K
}

/*
 * Reserved for the benchmark log (not referenced by any section): the
 * hardware firmware writes its console mirror + done flag here via absolute
 * addresses (src/hw.rs::benchlog_*), and OpenOCD dump_image's it for the
 * host parser (scripts/parse_bench_dump.py).
 *
 * 0x2001C000  magic  u32  "CSBL" = 0x4353424C (LE)
 * 0x2001C004  status u32  1 = all gates passed, 2 = gate failed
 * 0x2001C008  length u32  log bytes at +0x10
 * 0x2001C010  log    u8[length]
 *
 * Region: SRAM2 (16K, contiguous with SRAM1), hence RAM above is capped at
 * 112K so the linker can never place data or heap inside the log region.
 */
