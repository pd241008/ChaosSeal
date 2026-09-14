//! Bare-metal STM32F4 hardware bring-up (feature `hw`).
//!
//! Register sequences ported from the Midas artifact's TFLM firmware
//! (`../Midas/mcu/fw/src/{system_init.c,main.cc}`), same Discovery-F407VG
//! board: HSE 8 MHz -> PLL 168 MHz (5 flash wait states + caches), USART2
//! console at 115200 8N1 on PA2/PA3 (AF7), status LEDs on PD12..PD15, and
//! the DWT cycle counter for true cycle-domain timing.
//!
//! All MMIO is raw pointers; every write sequence mirrors Midas's C code
//! (same masks, same ordering). On hardware this firmware then runs at
//! 168 MHz exactly like Midas, so `cycles/168e6` is true seconds and the
//! benchmark reports genuine DWT cycle counts.

#![allow(dead_code)]

// ---------------------------------------------------------------------------
// Register block base addresses (STM32F407 reference manual RM0090)
// ---------------------------------------------------------------------------
const RCC_BASE: usize = 0x4002_3800;
const GPIOA_BASE: usize = 0x4002_0000;
const GPIOD_BASE: usize = 0x4002_0C00;
const USART2_BASE: usize = 0x4000_4400;
const FLASH_BASE: usize = 0x4002_3C00; // FLASH_ACR lives here on F4
const CORESIGHT_BASE: usize = 0xE000_ED00;

// RCC
const RCC_CR: *mut u32 = (RCC_BASE) as *mut u32;
const RCC_PLLCFGR: *mut u32 = (RCC_BASE + 0x04) as *mut u32;
const RCC_CFGR: *mut u32 = (RCC_BASE + 0x08) as *mut u32;
const RCC_AHB1ENR: *mut u32 = (RCC_BASE + 0x30) as *mut u32;
const RCC_APB1ENR: *mut u32 = (RCC_BASE + 0x40) as *mut u32;
// FLASH
const FLASH_ACR: *mut u32 = (FLASH_BASE) as *mut u32;
// GPIO
const GPIOA_MODER: *mut u32 = (GPIOA_BASE + 0x00) as *mut u32;
const GPIOA_AFRL: *mut u32 = (GPIOA_BASE + 0x20) as *mut u32;
const GPIOD_MODER: *mut u32 = (GPIOD_BASE + 0x00) as *mut u32;
const GPIOD_BSRR: *mut u32 = (GPIOD_BASE + 0x18) as *mut u32;
// USART2
const USART_SR: *mut u32 = (USART2_BASE + 0x00) as *mut u32;
const USART_DR: *mut u32 = (USART2_BASE + 0x04) as *mut u32;
const USART_BRR: *mut u32 = (USART2_BASE + 0x08) as *mut u32;
const USART_CR1: *mut u32 = (USART2_BASE + 0x0C) as *mut u32;
// CoreSight / DWT (DWT is its own block at 0xE0001000, NOT ED00+0x1000)
const DEMCR: *mut u32 = (CORESIGHT_BASE + 0xFC) as *mut u32; // 0xE000EDFC
const DWT_CTRL: *mut u32 = 0xE000_1000 as *mut u32;
const DWT_CYCCNT: *mut u32 = 0xE000_1004 as *mut u32;
// DCache clean-by-MVA (RM0090 C7.7): the debugger's AHB-AP reads SRAM
// directly and does NOT snoop the F407's write-back DCache, so every byte
// published for dump_image must be cleaned to SRAM first (Midas never hit
// this because its C code placed its arrays in no-cache-able sections via
// scatter-loading; we clean explicitly instead).
const DCCMVAC: *mut u32 = 0xE000_EF5C as *mut u32;

// RCC_CR bits
const RCC_CR_HSION: u32 = 1 << 0;
const RCC_CR_HSIRDY: u32 = 1 << 1;
const RCC_CR_HSEON: u32 = 1 << 16;
const RCC_CR_HSERDY: u32 = 1 << 17;
const RCC_CR_PLLON: u32 = 1 << 24;
const RCC_CR_PLLRDY: u32 = 1 << 25;
// RCC_CFGR bits
const RCC_CFGR_SW_PLL: u32 = 0b10;
const RCC_CFGR_SWS_PLL: u32 = 0b10 << 2;
const RCC_CFGR_PPRE1_DIV2: u32 = 0b100 << 10;
const RCC_PLLCFGR_HSE: u32 = 1 << 22;
// FLASH_ACR
const FLASH_ACR_LATENCY_5WS: u32 = 5;
const FLASH_ACR_PRFTEN: u32 = 1 << 8;
const FLASH_ACR_ICEN: u32 = 1 << 9;
const FLASH_ACR_DCEN: u32 = 1 << 10;
// USART_CR1
const USART_CR1_UE: u32 = 1 << 13;
const USART_CR1_TE: u32 = 1 << 3;
const USART_CR1_RE: u32 = 1 << 2;
const USART_SR_TXE: u32 = 1 << 7;
// DEMCR / DWT
const DEMCR_TRCENA: u32 = 1 << 24;
const DWT_CTRL_CYCCNTENA: u32 = 1;

#[inline(always)]
fn r(reg: *mut u32) -> u32 {
    unsafe { reg.read_volatile() }
}

#[inline(always)]
fn w(reg: *mut u32, v: u32) {
    unsafe { reg.write_volatile(v) }
}

#[inline(always)]
fn rmw(reg: *mut u32, mask: u32, set: u32) {
    unsafe {
        let v = reg.read_volatile();
        reg.write_volatile((v & !mask) | set);
    }
}

/// Spin on `reg` until any of `mask` becomes set (with a bounded timeout so a
/// hardware fault hangs the LED, not a silent lockup).
fn wait_set(reg: *mut u32, mask: u32) {
    let mut guard: u32 = 0xFFFF_FFFF;
    while r(reg) & mask == 0 {
        core::hint::spin_loop();
        guard -= 1;
        if guard == 0 {
            led_error(); // stuck: signal red and halt here
        }
    }
}

/// System clock bring-up, verbatim from Midas `system_init.c`:
/// HSE 8 MHz -> PLL (M=8, N=336, P=2, Q=7) -> 168 MHz SYSCLK,
/// APB1 = 42 MHz (USART2 clock domain), 5 flash wait states + caches.
pub fn clock_init_168mhz() {
    // FPU enable (CP10/CP11 full access) — belt and suspenders; cortex-m-rt
    // may or may not have enabled it depending on target features.
    const SCB_CPACR: *mut u32 = (CORESIGHT_BASE + 0x88) as *mut u32;
    rmw(SCB_CPACR, 0xF << 20, 0xF << 20);

    // DWT cycle counter enable (also re-done in dwt_enable()).
    rmw(DEMCR, DEMCR_TRCENA, DEMCR_TRCENA);
    w(DWT_CYCCNT, 0);
    rmw(DWT_CTRL, DWT_CTRL_CYCCNTENA, DWT_CTRL_CYCCNTENA);

    // Safe starting point: HSI on.
    rmw(RCC_CR, RCC_CR_HSION, RCC_CR_HSION);
    let _ = r(RCC_CR) & RCC_CR_HSIRDY;

    // PLL config: HSE source, M=8, N=336, P=2, Q=7 (Midas constants).
    w(
        RCC_PLLCFGR,
        8
            | (336u32 << 6)
            | (((2u32 >> 1) - 1) << 16)
            | RCC_PLLCFGR_HSE
            | (7u32 << 24),
    );

    // Enable HSE and wait ready.
    rmw(RCC_CR, RCC_CR_HSEON, RCC_CR_HSEON);
    wait_set(RCC_CR, RCC_CR_HSERDY);

    // Flash latency for 168 MHz: 5 WS + prefetch + I/D caches.
    w(
        FLASH_ACR,
        FLASH_ACR_PRFTEN | FLASH_ACR_ICEN | FLASH_ACR_DCEN | FLASH_ACR_LATENCY_5WS,
    );

    // AHB /1, APB1 /2 (42 MHz), APB2 /1.
    rmw(RCC_CFGR, 0xF << 10, RCC_CFGR_PPRE1_DIV2);

    // Enable PLL, wait, switch SYSCLK, wait for switch.
    rmw(RCC_CR, RCC_CR_PLLON, RCC_CR_PLLON);
    wait_set(RCC_CR, RCC_CR_PLLRDY);
    rmw(RCC_CFGR, 0b11, RCC_CFGR_SW_PLL);
    wait_set(RCC_CFGR, RCC_CFGR_SWS_PLL);
}

/// DWT cycle counter enable (called again after clock init for ordering).
pub fn dwt_enable() {
    rmw(DEMCR, DEMCR_TRCENA, DEMCR_TRCENA);
    rmw(DWT_CTRL, DWT_CTRL_CYCCNTENA, DWT_CTRL_CYCCNTENA);
}

/// True CPU cycles since DWT enable (hardware only; frozen under QEMU).
pub fn dwt_cycles() -> u32 {
    r(DWT_CYCCNT)
}

/// GPIO + USART2 @ 115200 8N1 on PA2(TX)/PA3(RX), verbatim from Midas
/// `main.cc:uart2_init()`. Enables GPIOA/GPIOD/USART2 peripheral clocks.
pub fn uart2_init() {
    // Peripheral clocks: GPIOA (USART2), GPIOD (LEDs), USART2 on APB1.
    rmw(RCC_AHB1ENR, (1 << 0) | (1 << 3), (1 << 0) | (1 << 3)); // A, D
    rmw(RCC_APB1ENR, 1 << 17, 1 << 17); // USART2

    // PA2 = AF7 (USART2 TX), PA3 = AF7 (USART2 RX)
    rmw(GPIOA_MODER, (3 << 4) | (3 << 6), (2 << 4) | (2 << 6));
    rmw(GPIOA_AFRL, (0xF << 8) | (0xF << 12), (7 << 8) | (7 << 12));

    // BRR for 115200 @ 42 MHz APB1: 364.58 -> mantissa=36, frac=10 (Midas)
    w(USART_BRR, (36 << 4) | 10);
    w(USART_CR1, USART_CR1_UE | USART_CR1_TE | USART_CR1_RE);
}

pub fn uart_putc(c: u8) {
    while r(USART_SR) & USART_SR_TXE == 0 {
        core::hint::spin_loop();
    }
    w(USART_DR, c as u32);
}

/// LEDs PD12=green PD13=orange PD14=red PD15=blue, verbatim from Midas.
pub fn led_init() {
    rmw(
        GPIOD_MODER,
        (3 << 24) | (3 << 26) | (3 << 28) | (3 << 30),
        (1 << 24) | (1 << 26) | (1 << 28) | (1 << 30),
    );
}

pub fn led_on(n: u32) {
    w(GPIOD_BSRR, 1 << (12 + n));
}

pub fn led_off(n: u32) {
    w(GPIOD_BSRR, 1 << (12 + n + 16));
}

/// Red LED steady + halt: unrecoverable hardware bring-up failure.
fn led_error() -> ! {
    led_on(2);
    loop {
        core::hint::spin_loop();
    }
}

/// Panic handler for the hardware build (Midas semantics: red LED PD14
/// steady, then halt). Replaces `panic-halt` so any unexpected panic —
/// including pre-console bring-up faults — is visible on the board. If the
/// panic fires before `led_init()`, GPIO is not clocked yet and the LED
/// silently stays off; the bounded-timeout `wait_set` paths call
/// `led_error()` directly after `led_init()` has run.
#[cfg(all(feature = "hw", not(feature = "qemu")))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    led_on(2);
    loop {
        core::hint::spin_loop();
    }
}

// ---------------------------------------------------------------------------
// SRAM bench log (Midas openocd/dump_image pattern)
// ---------------------------------------------------------------------------
// The hardware console (USART2 on PA2) needs a USB-serial adapter that the
// ST-LINK/V2 cannot provide. Instead, like Midas's openocd/dump_image flow,
// the firmware mirrors every console line into a reserved SRAM2 region with
// a magic header and a status word; OpenOCD resets/runs the board, polls the
// status, halts and `dump_image`s the region for the host parser
// (scripts/parse_bench_dump.py). No serial adapter needed.

/// Magic in the log header: "CSBL" little-endian.
const BENCHLOG_MAGIC: u32 = 0x4353_424C;
const BENCHLOG_BASE: usize = 0x2001_C000; // SRAM2 (reserved in memory.x)
const BENCHLOG_DATA: usize = BENCHLOG_BASE + 0x10;
const BENCHLOG_MAX: usize = 4096;

/// Status words published in the header (schema also in scripts/parse_bench_dump.py).
const BENCHLOG_STATUS_OK: u32 = 1;
const BENCHLOG_STATUS_FAIL: u32 = 2;

static mut BENCHLOG_POS: usize = 0;

/// Zero the header and position (called at console init so a soft-reset
/// re-run never appends to a stale log). Body bytes are cleaned per-line
/// as they are written.
pub fn benchlog_reset() {
    unsafe {
        BENCHLOG_POS = 0;
        for i in 0..(BENCHLOG_MAX + 0x10) {
            (BENCHLOG_BASE as *mut u8).add(i).write_volatile(0);
        }
        let mut a = BENCHLOG_BASE;
        while a < BENCHLOG_BASE + 0x10 + 32 {
            dcache_clean_mva(a);
            a += 32;
        }
    }
}

/// Clean one address's DCache line to SRAM (DMB + DCCMVAC + DSB per RM0090).
#[inline(always)]
fn dcache_clean_mva(p: usize) {
    unsafe {
        core::arch::asm!("dmb");
        core::ptr::write_volatile(DCCMVAC, (p & !0x1F) as u32);
        core::arch::asm!("dsb");
    }
}

fn benchlog_write_u32(addr: usize, v: u32) {
    unsafe { (addr as *mut u32).write_volatile(v) };
    dcache_clean_mva(addr);
}

/// Append a line to the SRAM log (no trailing newline needed; parser splits
/// on \n and the header carries the exact length).
pub fn benchlog_line(line: &str) {
    unsafe {
        let pos = BENCHLOG_POS;
        let room = BENCHLOG_MAX.saturating_sub(pos);
        let n = line.len().min(room.saturating_sub(1));
        if n > 0 {
            let dst = (BENCHLOG_DATA + pos) as *mut u8;
            core::ptr::copy_nonoverlapping(line.as_ptr(), dst, n);
            // Clean the full line, cache-line granular at both ends.
            let start = (BENCHLOG_DATA + pos) & !0x1F;
            let end = BENCHLOG_DATA + pos + n;
            let mut a = start;
            while a < end + 32 {
                dcache_clean_mva(a);
                a += 32;
            }
            BENCHLOG_POS = pos + n + 1; // +1 for the parser's \n separator
            let _ = core::ptr::write_volatile((BENCHLOG_DATA + pos + n) as *mut u8, b'\n');
            dcache_clean_mva(BENCHLOG_DATA + pos + n);
        }
    }
}

/// Publish the header: magic + final status + exact log length. The status
/// store is the done flag OpenOCD polls; it must go out last.
pub fn benchlog_publish(ok: bool) {
    unsafe {
        let len = core::ptr::addr_of!(BENCHLOG_POS).read();
        benchlog_write_u32(BENCHLOG_BASE, BENCHLOG_MAGIC);
        benchlog_write_u32(
            BENCHLOG_BASE + 4,
            if ok { BENCHLOG_STATUS_OK } else { BENCHLOG_STATUS_FAIL },
        );
        benchlog_write_u32(BENCHLOG_BASE + 8, len as u32);
    }
}
