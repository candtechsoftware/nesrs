use crate::{console::Console, memory::Memory};
use image::Rgba;

pub struct PPU {
    memory: Box<dyn Memory>,
    console: Console,

    cycle: i32,
    scanline: i32,
    frame: u64,

    palete_data: [u8; 32],
    name_table_data: [u8; 2048],
    oam_data: [u8; 256],
    front: Rgba<u16>,
    back: Rgba<u16>,

    // PPU Registers
    v: u16,
    t: u16,
    x: u8,
    w: u8,
    f: u8,

    register: u8,

    // NMI Flags
    nmi_occurred: bool,
    nmi_output: bool,
    nmi_prev: bool,
    nmi_delay: u8,

    // Background temp variables
    name_table_byte: u8,
    attr_table_byte: u8,
    low_tile_byte: u8,
    high_tile_byte: u8,
    tile_data: u64,

    // Sprite temp variables
    sprite_count: i32,
    sprite_patterns: [u32; 8],
    sprite_position: [u32; 8],
    sprite_priorities: [u32; 8],
    sprite_indexes: [u32; 8],

    // $2000 PPUCTRL
    flag_name_table: u8,
    flag_increment: u8,
    flag_sprite_table: u8,
    flag_background_table: u8,
    flag_sprite_size: u8,
    flag_master_slave: u8,

    // $2000 PPUMASK
    flag_gray_scale: u8,
    flag_show_left_background: u8,
    flag_show_left_sprites: u8,
    flag_show_background: u8,
    flag_show_sprites: u8,
    flag_red_tint: u8,
    flag_green_tint: u8,
    flag_blue_tint: u8,

    // $2002 PPUSTATUS
    flag_sprite_zero_hit: u8,
    flag_sprite_overflow: u8,

    // $2003 OAMADDR
    oam_addr: u8,

    // $2007 PPUDATA
    buffer_data: u8,
}

impl Default for PPU {
    fn default() -> Self {
        let front: Rgba<u16> = Rgba([0, 0, 256, 240]);
        let back: Rgba<u16> = Rgba([0, 0, 256, 240]);
        Self {
            console: Console::default(),
            front,
            back,
            ..Default::default()
        }
    }
}

impl PPU {
    fn new(console: Console) -> Self {
        Self {
            console,
            ..Default::default()
        }
    }
    fn reset(mut self) {
        self.cycle = 340;
        self.scanline = 240;
        self.frame = 0;
        self.write_control(0);
        self.write_mask(0);
        self.write_oam_addr(0);
    }

    fn read_palette(&mut self, mut addr: u16) -> u8 {
        if addr >= 16 && addr % 4 == 0 {
            addr -= 16
        }
        self.palete_data[addr as usize]
    }

    fn write_palette(&mut self, mut addr: u16, value: u8) {
        if addr >= 16 && addr % 4 == 0 {
            addr -= 16
        }
        self.palete_data[addr as usize] = value
    }

    fn read_register(&mut self, addr: u16) -> u8 {
        match addr {
            0x2002 => {
                return self.read_status();
            }
            0x2004 => {
                return self.read_oam_data();
            }
            0x2007 => {
                return self.read_data();
            }
            _ => return 0 as u8,
        }
    }

    fn write_register(&mut self, addr: u16, value: u8) {
        self.register = value;
        match addr {
            0x2000 => return self.write_control(value),
            0x2001 => return self.write_mask(value),
            0x2003 => return self.write_oam_addr(value),
            0x2004 => return self.write_oam_data(value),
            0x2005 => return self.write_scroll(value),
            0x2006 => return self.write_addr(value),
            0x2007 => return self.write_data(value),
            0x4014 => return self.write_dma(value),
        }
    }

    // $2000: PPUCTRL
    fn write_control(&mut self, value: u8) {
        self.flag_name_table = (value >> 0) & 3;
        self.flag_increment = (value >> 2) & 1;
        self.flag_sprite_table = (value >> 3) & 1;
        self.flag_background_table = (value >> 4) & 1;
        self.flag_sprite_size = (value >> 5) & 1;
        self.flag_master_slave = (value >> 6) & 1;
        self.nmi_output = (value >> 7) & 1 == 1;
        self.nmi_change();
        self.t = (self.t & 0xF3FF) | ((value as u16 & 0x03) << 10)
    }
    // $2001: PPUMASK
    fn write_mask(&mut self, value: u8) {
        self.flag_gray_scale = (value >> 0) & 1;
        self.flag_show_left_background = (value >> 1) & 1;
        self.flag_show_left_sprites = (value >> 2) & 1;
        self.flag_show_background = (value >> 3) & 1;
        self.flag_show_sprites = (value >> 4) & 1;
        self.flag_red_tint = (value >> 5) & 1;
        self.flag_blue_tint = (value >> 6) & 1;
        self.flag_blue_tint = (value >> 7) & 1;
    }

    // $2002: PPUSTATUS
    fn read_status(&mut self) -> u8 {
        let mut result = self.register & 0x1F;
        result |= self.flag_sprite_overflow;
        result |= self.flag_sprite_zero_hit;
        result |= self.flag_sprite_zero_hit;

        if self.nmi_occurred {
            result |= 1 << 7;
        }

        self.nmi_occurred = false;
        self.nmi_change();

        self.w = 0;
        return result;
    }

    // $2003: OAMADDR
    fn write_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    // $2004: OAMDATA (read)
    fn read_oam_data(&self) -> u8 {
        let mut data = self.oam_data[self.oam_addr as usize] as u8;
        if (self.oam_addr & 0x03) == 0x02 {
            data = data & 0xE3;
        }
        data
    }

    // $2004: OAMDATA (write)
    fn write_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr += 1
    }

    // $2005: PPUSCROLL
    fn write_scroll(&mut self, value: u8) {
        if self.w == 0 {
            // t: ........ ...HGFED = d: HGFED...
            // x:               CBA = d: .....CBA
            // w:                   = 1
            self.t = (self.t & 0x8FFE0) | (value as u16 >> 3);
            self.x = value & 0x07;
            self.w = 1;
        } else {
            // t: .CBA..HG FED..... = d: HGFEDCBA
            // w:                   = 0
            self.t = (self.t & 0x8FFF) | ((value as u16 & 0x07) << 12);
            self.t = (self.t & 0xFC1F) | (((value as u16) & 0xF8) << 2);
            self.w = 0;
        }
    }

    // $2006: PPUADDR
    fn write_addr(&mut self, value: u8) {
        if self.w == 0 {
            // t: ..FEDCBA ........ = d: ..FEDCBA
            // t: .X...... ........ = 0
            // w:                   = 1
            self.t = (self.t & 0x80FF) | ((value as u16 & 0x3F) << 8);
            self.w = 1;
        } else {
            // t: ........ HGFEDCBA = d: HGFEDCBA
            // v                    = t
            // w:                   = 0
            self.t = (self.t & 0xFF00) | value as u16;
            self.v = self.t;
            self.w = 0;
        }
    }

    // $2007: PPUDATA (read)
    fn read_data(&mut self) -> u8 {
        let mut value = self.memory.read(self.v);

        if self.v % 0x4000 < 0x3F00 {
            let buffered = self.buffer_data;
            self.buffer_data = value;
            value = buffered;
        } else {
            self.buffer_data = self.memory.read(self.v - 0x1000);
        }

        if self.flag_increment == 0 {
            self.v += 1;
        } else {
            self.v += 32;
        }

        value
    }

    // $2007: PPUDATA (write)
    fn write_data(&mut self, value: u8) {
        self.memory.write(self.v, value);
        if self.flag_increment == 0 {
            self.v += 1;
        } else {
            self.v += 32;
        }
    }

    // $4014: OAMDMA
    fn write_dma(&mut self, value: u8) {
        // TODO: Implemet CPU
        println!("{}", value);
    }

    fn nmi_change(&mut self) {}
}
