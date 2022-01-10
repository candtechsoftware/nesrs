use crate::console::Console;

pub trait Memory {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
}

#[derive(Copy, Clone)]
pub struct CPUMemory {
    console: Console,
}

impl CPUMemory {
    fn new(console: Console) -> Self {
        Self { console }
    }
}

impl Memory for CPUMemory {
    fn read(&self, addr: u16) -> u8 {
        0 as u8
    }

    fn write(&mut self, addr: u16, value: u8) {}
}
