#![no_std]
#![no_main]

#[no_mangle]
#[link_section = ".text.boot"]
unsafe extern "C" fn _start() -> ! {
    unsafe {
        core::arch::asm!(
            "wfi",
            options(noreturn)
        )
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}