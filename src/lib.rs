#![no_std]
#![feature(abi_avr_interrupt)]
#![feature(llvm_asm)]
#![feature(never_type)]
#![feature(maybe_uninit_ref)]
#![feature(const_in_array_repeat_expressions)]

mod driver;
mod executor;
mod timers;
mod waiter;

pub use driver::Driver;
pub use executor::Executor;
pub use waiter::Waiter;
pub use timers::{
    init_timers,
    micros,
    micros_no_interrupt,
    millis,
};
