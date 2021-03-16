#[cfg(feature = "atmega328p")]
use atmega328p_hal::pac::TC0;
#[cfg(feature = "atmega8u2")]
use atmega8u2p_hal::pac::TC0;
use avr_hal_generic::{
    avr_device,
    avr_device::interrupt::free as critical_section,
};
use core::task::Waker;
use minarray::MinArray;

static mut TIMER0_OVF_COUNT: u32 = 0;
static mut ELAPSED_MS: u32 = 0;

// Using prescale_64 gives 64 / 16000000 = 4us per tick;
// The timer overflows every 4 * 256 = 1024us
const TIMER0_TICK_US: u32 = 4;
const TIMER0_OVF_US: u32 = 1024;
const TIMER0_TICKS_PER_MS: u8 = 250; // 1000us / (4us per tick) = 250 ticks/ms

static mut WAITERS: MinArray<Waker> = MinArray::new();

unsafe fn r_tcnt0() -> u32 {
    (*TC0::ptr()).tcnt0.read().bits().into()
}

unsafe fn r_tifr0() -> u32 {
   (*TC0::ptr()).tifr0.read().bits().into()
}

pub fn init_timers() {
    let t0 = TC0::ptr();
    unsafe {
        (*t0).tccr0b.write(|w| w.cs0().prescale_64());
        (*t0).tcnt0.write(|w| { w.bits(0) });
        (*t0).timsk0.write(|w| { w.bits(0x03) }); // enable overflow interrupt and COMPA interrupt
        (*t0).ocr0a.write(|w| { w.bits(TIMER0_TICKS_PER_MS) });
    }
}

// In *theory* this wouldn't overflow for (255 * 1024 + 4294967295 * 1024)us, but since
// it can only return a 32-bit integer, it actually wraps around after about 71 minutes.
//
// Bummer.
pub fn micros() -> u32 {
    critical_section(|_| micros_no_interrupt())
}

// Call this function if you're already in a disabled-interrupt context to avoid unnecessary
// sei/cli (interrupt enable/disable) instructions
pub fn micros_no_interrupt() -> u32 {
    unsafe {
        // If the TIMER0_OVF interrupt fires and interrupts are disabled, the TCNT0 register will
        // still overflow but the value in the TIMER0_OVF_COUNT will be incorrect, leading to this
        // function returning incorrect values.  We still could get incorrect values if the
        // interrupt fires between reading TIFR0 and TCNT0, but this (seems to) happen rarely.  In
        // this case we will add an extra 1024us into the timer, which will be rectified the next
        // time the function is called.
        let count0 = r_tcnt0();
        let extra_ovf = r_tifr0() & 1;
        count0 * TIMER0_TICK_US + (TIMER0_OVF_COUNT + extra_ovf) * TIMER0_OVF_US
    }
}

// This will overflow after about 49 days
pub fn millis() -> u32 {
    critical_section(|_| unsafe { ELAPSED_MS })
}

pub fn register_timed_waker(trigger_time_ms: u32, waker: Waker) {
    critical_section(|_| unsafe {
        match WAITERS.push(trigger_time_ms, waker) {
            // Calling .unwrap() on the above makes the entire thing break; I think perhaps
            // unwrap while in a Future is somehow broken in the rust compiler...?
            _ => (),
        }
    });
}

#[cfg_attr(feature = "atmega328p", avr_device::interrupt(atmega328p))]
#[cfg_attr(feature = "atmega8u2", avr_device::interrupt(atmega8u2))]
unsafe fn TIMER0_OVF() {
    TIMER0_OVF_COUNT += 1;
}

#[cfg_attr(feature = "atmega328p", avr_device::interrupt(atmega328p))]
#[cfg_attr(feature = "atmega8u2", avr_device::interrupt(atmega8u2))]
unsafe fn TIMER0_COMPA() {
    ELAPSED_MS += 1;

    if ELAPSED_MS > WAITERS.min {
        for (_, waker) in WAITERS.take_less_than(ELAPSED_MS) {
            waker.wake();
        }
    }

    let OCR0A = &(*TC0::ptr()).ocr0a as *const _ as *mut u8;
    *OCR0A += TIMER0_TICKS_PER_MS; // Modular arithmetic works!  :D
}
