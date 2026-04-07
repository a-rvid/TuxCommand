#![no_std]
#![no_main]


#[cfg(debug_assertions)]
mod debug;

#[cfg(debug_assertions)]
use crate::debug::print;

#[cfg(not(panic = "immediate-abort"))]
use core::panic::PanicInfo;

#[cfg(not(panic = "immediate-abort"))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}

#[unsafe(no_mangle)]
extern "C" fn main() -> u8 {
    #[cfg(debug_assertions)]
    {
        let string: &str = "&str test";
        println!("Tuxmux client, do not use in production\n{}", string);
        for i in 0..5 {
            println!("int loop {}", i);
        }
    }
    0
}