use riscv::register::sstatus;

// enable device interrupts
#[inline(always)]
pub fn interrupt_on() {
    unsafe {
        sstatus::set_sie();
    }
}

// disable device interrupts
#[allow(unused)]
#[inline(always)]
pub fn interrupt_off() {
    unsafe {
        sstatus::clear_sie();
    }
}

// are device interrupts enabled?
#[allow(unused)]
#[inline(always)]
pub fn interrupt_get() -> bool {
    sstatus::read().sie()
}
