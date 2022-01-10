pub static CPUFREQ: usize = 1789773;

pub enum IRQ {
    Normal,
    NMI,
    RESET,
}

pub enum AddressMode {
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Accumulator,
    Immediate,
    Implied,
    IndexedIndirect,
    Indirect,
    IndirectIndexed,
    Relative,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
}

pub struct CPU {}
