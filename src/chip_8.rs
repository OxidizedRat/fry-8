use bitvec::order::Msb0;
use bitvec::vec::BitVec;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::u8;
mod key_sprites;
use key_sprites::*;
pub struct Chip8 {
    memory: [u8; 4097],
    display: Output,
    registers: Registers,
    stack: Vec<u16>,
    pub keyboard: Keyboard,
    rom_loaded: bool,
}
impl Chip8 {
    //initialize all the variables we need
    pub fn init() -> Chip8 {
        let memory: [u8; 4097] = [0; 4097];
        let sprites = Vec::new();
        let key_sprites = HashMap::new();
        let display = Output {
            sprites,
            key_sprites,
        };
        let registers = Registers::new();
        let stack: Vec<u16> = Vec::with_capacity(16);
        let keyboard = Keyboard { keycode: None };
        let rom_loaded = false;

        let mut chip8 = Chip8 {
            memory,
            display,
            registers,
            stack,
            keyboard,
            rom_loaded,
        };
        chip8.load_sprite_data();
        return chip8;
    }
    fn load_sprite_data(&mut self) {
        let mut index = 0;
        for sprite in KEY_SPRITES {
            for byte in sprite {
                self.memory[index] = byte;
                index += 1;
            }
        }
        let key_sprites: HashMap<u8, u16> = HashMap::from([
            (0x0, 0),
            (0x1, 5),
            (0x2, 10),
            (0x3, 15),
            (0x4, 20),
            (0x5, 25),
            (0x6, 30),
            (0x7, 35),
            (0x8, 40),
            (0x9, 45),
            (0xA, 50),
            (0xB, 55),
            (0xC, 60),
            (0xD, 65),
            (0xE, 70),
            (0xF, 75),
        ]);
    }
    //copy a rom file to memory, starting at 0x200 memory address
    pub fn load(&mut self, rom: &std::path::Path) -> Result<(), ChipError> {
        //ensure rom will fit in memory
        let buffer = match std::fs::read(rom) {
            Ok(buf) => buf,
            Err(why) => return Err(ChipError::IOError(why)),
        };
        let len = buffer.len();
        if len > 0xdff {
            return Err(ChipError::RomTooLarge(len - 0xdff));
        }
        //copy rom to memory
        let rom_start: usize = 0x200;
        let mut i = 0;
        for byte in buffer.iter() {
            self.memory[rom_start + i] = *byte;
            i += 1;
        }
        self.registers.program_counter = 0x200;
        self.rom_loaded = true;
        Ok(())
    }
    //fetch current instruction and increment pc
    fn fetch(&mut self) -> Result<u16, ChipError> {
        let pc: usize = self.registers.program_counter.try_into().unwrap();
        if pc >= 0xfff || pc < 0x200 {
            return Err(ChipError::AddressOutofBounds);
        }
        let byte1 = self.memory[pc];
        let byte2 = self.memory[pc + 1];
        self.registers.program_counter += 2;
        Ok(u16::from_be_bytes([byte1, byte2]))
    }
    //decode instruction
    fn decode(instruction: u16) -> Instruction {
        match instruction >> 12 {
            0x0 => match instruction {
                0x00E0 => return Instruction::ClearScreen,
                0x00EE => return Instruction::Return,
                _ => return Instruction::SYSaddr,
            },
            0x1 => {
                let addr = (instruction << 4) >> 4;
                return Instruction::Jump(addr);
            }
            0x2 => {
                let addr = (instruction << 4) >> 4;
                return Instruction::Call(addr);
            }
            0x3 => {
                let byte: u8 = ((instruction << 8) >> 8) as u8;
                let register: u8 = ((instruction << 4) >> 12) as u8;
                return Instruction::SkipEqualByte(register, byte);
            }
            0x4 => {
                let byte: u8 = ((instruction << 8) >> 8) as u8;
                let register: u8 = ((instruction << 4) >> 12) as u8;
                return Instruction::SkipNotEqualByte(register, byte);
            }
            0x5 => {
                let reg1: u8 = ((instruction << 4) >> 12) as u8;
                let reg2: u8 = ((instruction << 8) >> 12) as u8;
                return Instruction::SkipEqualReg(reg1, reg2);
            }
            0x6 => {
                let byte = ((instruction << 8) >> 8) as u8;
                let register: u8 = ((instruction << 4) >> 12) as u8;
                return Instruction::LoadByte(register, byte);
            }
            0x7 => {
                let byte = ((instruction << 8) >> 8) as u8;
                let register: u8 = ((instruction << 4) >> 12) as u8;
                return Instruction::AddByte(register, byte);
            }
            0x8 => {
                let reg1 = ((instruction << 4) >> 12) as u8;
                let reg2 = ((instruction << 8) >> 12) as u8;
                match (instruction << 12) >> 12 {
                    0x0 => return Instruction::LoadReg(reg1, reg2),
                    0x1 => return Instruction::OR(reg1, reg2),
                    0x2 => return Instruction::AND(reg1, reg2),
                    0x3 => return Instruction::XOR(reg1, reg2),
                    0x4 => return Instruction::AddReg(reg1, reg2),
                    0x5 => return Instruction::SubReg(reg1, reg2),
                    0x6 => return Instruction::ShiftRight(reg1),
                    0x7 => return Instruction::SubN(reg1, reg2),
                    0xE => return Instruction::ShiftLeft(reg1),
                    _ => return Instruction::Invalid,
                }
            }
            0x9 => {
                let reg1 = ((instruction << 4) >> 12) as u8;
                let reg2 = ((instruction << 8) >> 12) as u8;
                return Instruction::SkipNotEqualReg(reg1, reg2);
            }
            0xA => {
                let addr = (instruction << 4) >> 4;
                return Instruction::LoadI(addr);
            }
            0xB => {
                let addr = (instruction << 4) >> 4;
                return Instruction::JumpAdd(addr);
            }
            0xC => {
                let byte = ((instruction << 8) >> 8) as u8;
                let reg = ((instruction << 4) >> 12) as u8;
                return Instruction::Rand(reg, byte);
            }
            0xD => {
                let reg1 = ((instruction << 4) >> 12) as u8;
                let reg2 = ((instruction << 8) >> 12) as u8;
                let sprite_size = ((instruction << 12) >> 12) as u8;

                return Instruction::Draw(reg1, reg2, sprite_size);
            }
            0xE => {
                let reg1 = ((instruction << 4) >> 12) as u8;
                match (instruction << 8) >> 8 {
                    0x9E => return Instruction::SkipKey(reg1),
                    0xA1 => return Instruction::SkipNotKey(reg1),
                    _ => {
                        return Instruction::Invalid;
                    }
                }
            }
            0xF => {
                let reg1 = ((instruction << 4) >> 12) as u8;
                match (instruction << 8) >> 8 {
                    0x07 => return Instruction::GetDelay(reg1),
                    0x0A => return Instruction::WaitKey(reg1),
                    0x15 => return Instruction::SetDelay(reg1),
                    0x18 => return Instruction::SetSound(reg1),
                    0x1E => return Instruction::AddI(reg1),
                    0x29 => return Instruction::SetISprite(reg1),
                    0x33 => return Instruction::StoreBCD(reg1),
                    0x55 => return Instruction::StoreRegI(reg1),
                    0x65 => return Instruction::LoadRegI(reg1),
                    _ => return Instruction::Invalid,
                }
            }
            _ => return Instruction::Invalid,
        }
    }
    //execute instruction
    pub fn exec(&mut self, i: Instruction) -> Result<SDLDo, ChipError> {
        match i {
            Instruction::SYSaddr => return Ok(SDLDo::None),
            Instruction::ClearScreen => {
                self.display.sprites.clear();
                return Ok(SDLDo::ClearScreen);
            }
            Instruction::Return => {
                self.registers.program_counter = match self.stack.pop() {
                    Some(addr) => addr,
                    None => return Err(ChipError::AddressOutofBounds),
                };
                self.registers.stack_pointer -= 1;
                return Ok(SDLDo::None);
            }
            Instruction::Jump(addr) => {
                self.registers.program_counter = addr;
                return Ok(SDLDo::None);
            }
            Instruction::Call(addr) => {
                self.stack.push(self.registers.program_counter);
                self.registers.stack_pointer += 1;
                self.registers.program_counter = addr;
                return Ok(SDLDo::None);
            }
            Instruction::SkipEqualByte(reg1, byte) => {
                let reg = self.registers.get_vx(reg1)?;
                if reg == byte {
                    self.registers.program_counter += 2;
                }
                return Ok(SDLDo::None);
            }
            Instruction::SkipNotEqualByte(reg1, byte) => {
                let reg = self.registers.get_vx(reg1)?;
                if reg != byte {
                    self.registers.program_counter += 2;
                }
                return Ok(SDLDo::None);
            }
            Instruction::SkipEqualReg(reg1, reg2) => {
                let reg1 = self.registers.get_vx(reg1)?;
                let reg2 = self.registers.get_vx(reg2)?;
                if reg1 == reg2 {
                    self.registers.program_counter += 2;
                }
                return Ok(SDLDo::None);
            }
            Instruction::LoadByte(reg, byte) => {
                self.registers.set_vx(reg, byte)?;
                return Ok(SDLDo::None);
            }
            Instruction::AddByte(reg, byte) => {
                let reg_val = self.registers.get_vx(reg)? as u16;
                let added_val = reg_val + byte as u16;
                if added_val > 255 {
                    self.registers.vf = 1;
                }
                self.registers.set_vx(reg, added_val as u8)?;
                return Ok(SDLDo::None);
            }
            Instruction::LoadReg(reg1, reg2) => {
                let reg2_val = self.registers.get_vx(reg2)?;
                self.registers.set_vx(reg1, reg2_val)?;
                return Ok(SDLDo::None);
            }
            Instruction::OR(reg1, reg2) => {
                let reg1_val = self.registers.get_vx(reg1)?;
                let reg2_val = self.registers.get_vx(reg2)?;

                self.registers.set_vx(reg1, reg1_val | reg2_val)?;
                return Ok(SDLDo::None);
            }
            Instruction::AND(reg1, reg2) => {
                let reg1_val = self.registers.get_vx(reg1)?;
                let reg2_val = self.registers.get_vx(reg2)?;

                self.registers.set_vx(reg1, reg1_val & reg2_val)?;
                return Ok(SDLDo::None);
            }
            Instruction::XOR(reg1, reg2) => {
                let reg1_val = self.registers.get_vx(reg1)?;
                let reg2_val = self.registers.get_vx(reg2)?;
                self.registers.set_vx(reg1, reg1_val ^ reg2_val)?;
                return Ok(SDLDo::None);
            }
            Instruction::AddReg(reg1, reg2) => {
                let reg1_val = self.registers.get_vx(reg1)? as u16;
                let reg2_val = self.registers.get_vx(reg2)? as u16;
                let added_val = reg1_val + reg2_val;
                if added_val > 255 {
                    self.registers.vf = 1;
                } else {
                    self.registers.vf = 0;
                }
                self.registers.set_vx(reg1, added_val as u8)?;
                return Ok(SDLDo::None);
            }
            Instruction::SubReg(reg1, reg2) => {
                let reg1_val = self.registers.get_vx(reg1)?;
                let reg2_val = self.registers.get_vx(reg2)?;

                let (val, overflow) = reg1_val.overflowing_sub(reg2_val);

                if overflow {
                    self.registers.set_vx(0xf, 1)?;
                } else {
                    self.registers.set_vx(0xf, 0)?;
                }
                self.registers.set_vx(reg1, val)?;
                return Ok(SDLDo::None);
            }
            Instruction::ShiftRight(reg) => {
                let reg_val = self.registers.get_vx(reg)?;
                if reg_val.trailing_ones() > 0 {
                    self.registers.set_vx(0xf, 1)?;
                } else {
                    self.registers.set_vx(0xf, 0)?;
                }
                self.registers.set_vx(reg, reg_val >> 1)?;
                return Ok(SDLDo::None);
            }
            Instruction::SubN(reg1, reg2) => {
                let reg1_val = self.registers.get_vx(reg1)?;
                let reg2_val = self.registers.get_vx(reg2)?;
                let (val, overflow) = reg2_val.overflowing_sub(reg1_val);
                if overflow {
                    self.registers.set_vx(0xf, 1)?;
                } else {
                    self.registers.set_vx(0xf, 0)?;
                }
                self.registers.set_vx(reg1, val)?;
                return Ok(SDLDo::None);
            }
            Instruction::ShiftLeft(reg) => {
                let reg_val = self.registers.get_vx(reg)?;
                if reg_val.leading_ones() > 0 {
                    self.registers.set_vx(0xf, 1)?;
                } else {
                    self.registers.set_vx(0xf, 0)?;
                }
                self.registers.set_vx(reg, reg_val << 1)?;
                return Ok(SDLDo::None);
            }
            Instruction::SkipNotEqualReg(reg1, reg2) => {
                let reg1_val = self.registers.get_vx(reg1)?;
                let reg2_val = self.registers.get_vx(reg2)?;
                if reg1_val != reg2_val {
                    self.registers.program_counter += 2;
                }
                return Ok(SDLDo::None);
            }
            Instruction::LoadI(addr) => {
                self.registers.i = addr;
                return Ok(SDLDo::None);
            }
            Instruction::JumpAdd(addr) => {
                let add_addr = addr + self.registers.v0 as u16;

                self.registers.program_counter = add_addr;
                return Ok(SDLDo::None);
            }
            Instruction::Rand(reg, byte) => {
                let rng: u8 = rand::random();
                let val = byte & rng;
                self.registers.set_vx(reg, val)?;
                return Ok(SDLDo::None);
            }
            Instruction::Draw(reg1, reg2, size) => {
                let addr = self.registers.i as usize;
                let x = self.registers.get_vx(reg1)?;
                let y = self.registers.get_vx(reg2)?;
                let mut sprite_buf: Vec<u8> = Vec::new();
                for i in addr..(addr + size as usize) {
                    sprite_buf.push(self.memory[i]);
                }
                let sprite = Sprite::new(sprite_buf, x, y)?;
                let rects = sprite.into_rects();
                self.display.sprites.push(sprite);
                return Ok(SDLDo::Draw(rects));
            }
            Instruction::SkipKey(reg) => {
                let key = self.registers.get_vx(reg)?;
                if let Some(held_down) = self.keyboard.get_key() {
                    if key == held_down {
                        self.registers.program_counter += 2
                    }
                }
                return Ok(SDLDo::None);
            }
            Instruction::SkipNotKey(reg) => {
                let key = self.registers.get_vx(reg)?;
                if let Some(held_down) = self.keyboard.get_key() {
                    if key != held_down {
                        self.registers.program_counter += 2;
                    }
                }
                return Ok(SDLDo::None);
            }
            Instruction::GetDelay(reg) => {
                let delay_timer = self.registers.delay_timer;
                self.registers.set_vx(reg, delay_timer)?;
                return Ok(SDLDo::None);
            }
            Instruction::WaitKey(reg) => match self.keyboard.get_key() {
                Some(key) => {
                    self.registers.set_vx(reg, key)?;
                    return Ok(SDLDo::None);
                }
                None => {
                    self.registers.program_counter -= 1;
                    return Ok(SDLDo::None);
                }
            },
            Instruction::SetDelay(reg) => {
                let vx = self.registers.get_vx(reg)?;
                self.registers.delay_timer = vx;
                return Ok(SDLDo::None);
            }
            Instruction::SetSound(reg) => {
                let vx = self.registers.get_vx(reg)?;
                self.registers.sound_timer = vx;
                return Ok(SDLDo::None);
            }
            Instruction::AddI(reg) => {
                let vx = self.registers.get_vx(reg)? as u16;
                self.registers.i += vx;
                return Ok(SDLDo::None);
            }
            Instruction::SetISprite(reg) => {
                let vx = self.registers.get_vx(reg)?;
                if let Some(addr) = self.display.key_sprites.get(&vx) {
                    self.registers.i = *addr;
                }
                return Ok(SDLDo::None);
            }
            Instruction::StoreBCD(reg) => {
                let vx = self.registers.get_vx(reg)?;
                let first = vx / 100;
                let second: u8 = (vx / 10) - (first * 10);
                let third: u8 = vx - ((first * 100) + (second * 10));
                let addr = self.registers.i as usize;
                self.memory[addr] = first;
                self.memory[addr + 1] = second;
                self.memory[addr + 2] = third;
                return Ok(SDLDo::None);
            }
            Instruction::StoreRegI(reg) => {
                let mut addr = self.registers.i as usize;
                for x in 0..=reg {
                    let vx = self.registers.get_vx(x)?;
                    self.memory[addr] = vx;
                    addr += 1;
                }
                return Ok(SDLDo::None);
            }
            Instruction::LoadRegI(reg) => {
                let mut addr = self.registers.i as usize;
                for x in 0..=reg {
                    let byte = self.memory[addr];
                    addr += 1;
                    self.registers.set_vx(x, byte)?;
                }
                return Ok(SDLDo::None);
            }
            Instruction::Invalid => return Err(ChipError::InvalidInstruction),
        };
    }

    pub fn step(&mut self) -> Result<SDLDo, ChipError> {
        let raw_instruction = match self.fetch() {
            Ok(ins) => ins,
            Err(why) => return Err(why),
        };
        let instruction = Chip8::decode(raw_instruction);
        println!("{:?}", instruction);
        if self.registers.delay_timer != 0 {
            self.registers.delay_timer -= 1;
        }
        if self.registers.sound_timer != 0 {
            self.registers.sound_timer -= 1;
        }
        return Ok(self.exec(instruction)?);
    }
}
#[repr(transparent)]
pub struct Keyboard {
    keycode: Option<Keycode>,
}
impl Keyboard {
    pub fn get_key(&self) -> Option<u8> {
        if let Some(key) = self.keycode {
            match key {
                Keycode::Num1 => return Some(0x1),
                Keycode::Num2 => return Some(0x2),
                Keycode::Num3 => return Some(0x3),
                Keycode::Num4 => return Some(0xC),
                Keycode::Q => return Some(0x4),
                Keycode::W => return Some(0x5),
                Keycode::E => return Some(0x6),
                Keycode::R => return Some(0xD),
                Keycode::A => return Some(0x7),
                Keycode::S => return Some(0x8),
                Keycode::D => return Some(0x9),
                Keycode::F => return Some(0xE),
                Keycode::Z => return Some(0xA),
                Keycode::X => return Some(0x0),
                Keycode::C => return Some(0xB),
                Keycode::V => return Some(0xF),
                _ => return None,
            }
        } else {
            return None;
        }
    }
    pub fn set_key(&mut self, key: Keycode) {
        self.keycode = Some(key);
    }
}
pub struct Sprite {
    raw_bytes: Vec<u8>,
    x: u8,
    y: u8,
    y_max: u8,
}
impl Sprite {
    const X_MAX: u8 = 8;
    pub fn new(buff: Vec<u8>, x: u8, y: u8) -> Result<Sprite, ChipError> {
        if buff.len() > 15 {
            return Err(ChipError::InvalidSpriteSize);
        }
        let y_max = y + buff.len() as u8;
        Ok(Sprite {
            raw_bytes: buff,
            x,
            y,
            y_max,
        })
    }
    pub fn into_rects(&self) -> Vec<Rect> {
        let x_start = self.x as i32;
        let mut y = self.y as i32;
        let mut rects = Vec::new();
        for byte in self.raw_bytes.iter() {
            let mut x = x_start;
            let bits = BitVec::<u8, Msb0>::from_element(*byte);
            for bit in bits.iter().by_vals() {
                match bit {
                    true => {
                        let pixel = Rect::new(x, y, 1, 1);
                        rects.push(pixel);
                        x += 1;
                        continue;
                    }
                    false => {
                        x += 1;
                        continue;
                    }
                };
            }
            y += 1;
        }
        rects
    }
}

#[derive(Debug)]
pub enum Instruction {
    SYSaddr, //ignored
    ClearScreen,
    Return,
    Jump(u16),
    Call(u16),
    SkipEqualByte(u8, u8),
    SkipNotEqualByte(u8, u8),
    SkipEqualReg(u8, u8),
    LoadByte(u8, u8),
    AddByte(u8, u8),
    LoadReg(u8, u8),
    OR(u8, u8),
    AND(u8, u8),
    XOR(u8, u8),
    AddReg(u8, u8),
    SubReg(u8, u8),
    ShiftRight(u8),
    SubN(u8, u8),
    ShiftLeft(u8),
    SkipNotEqualReg(u8, u8),
    LoadI(u16),
    JumpAdd(u16),
    Rand(u8, u8),
    Draw(u8, u8, u8),
    SkipKey(u8),
    SkipNotKey(u8),
    GetDelay(u8),
    WaitKey(u8),
    SetDelay(u8),
    SetSound(u8),
    AddI(u8),
    SetISprite(u8),
    StoreBCD(u8),
    StoreRegI(u8),
    LoadRegI(u8),
    Invalid,
}
#[derive(Debug)]
pub enum SDLDo {
    Draw(Vec<Rect>),
    ClearScreen,
    None,
}
pub struct Output {
    sprites: Vec<Sprite>,
    key_sprites: HashMap<u8, u16>,
}

pub struct Registers {
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub i: u16,
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub v0: u8,
    pub v1: u8,
    pub v2: u8,
    pub v3: u8,
    pub v4: u8,
    pub v5: u8,
    pub v6: u8,
    pub v7: u8,
    pub v8: u8,
    pub v9: u8,
    pub va: u8,
    pub vb: u8,
    pub vc: u8,
    pub vd: u8,
    pub ve: u8,
    pub vf: u8,
}
impl Registers {
    pub fn new() -> Registers {
        Registers {
            program_counter: 0,
            stack_pointer: 0,
            i: 0,
            delay_timer: 0,
            sound_timer: 0,
            v0: 0,
            v1: 0,
            v2: 0,
            v3: 0,
            v4: 0,
            v5: 0,
            v6: 0,
            v7: 0,
            v8: 0,
            v9: 0,
            va: 0,
            vb: 0,
            vc: 0,
            vd: 0,
            ve: 0,
            vf: 0,
        }
    }
    pub fn set_vx(&mut self, reg_number: u8, value: u8) -> Result<(), ChipError> {
        match reg_number {
            0x0 => self.v0 = value,
            0x1 => self.v1 = value,
            0x2 => self.v2 = value,
            0x3 => self.v3 = value,
            0x4 => self.v4 = value,
            0x5 => self.v5 = value,
            0x6 => self.v6 = value,
            0x7 => self.v7 = value,
            0x8 => self.v8 = value,
            0x9 => self.v9 = value,
            0xA => self.va = value,
            0xB => self.vb = value,
            0xC => self.vc = value,
            0xD => self.vd = value,
            0xE => self.ve = value,
            0xF => self.vf = value,
            _ => return Err(ChipError::InvalidRegister),
        }
        Ok(())
    }
    pub fn get_vx(&self, reg_number: u8) -> Result<u8, ChipError> {
        match reg_number {
            0x0 => return Ok(self.v0),
            0x1 => return Ok(self.v1),
            0x2 => return Ok(self.v2),
            0x3 => return Ok(self.v3),
            0x4 => return Ok(self.v4),
            0x5 => return Ok(self.v5),
            0x6 => return Ok(self.v6),
            0x7 => return Ok(self.v7),
            0x8 => return Ok(self.v8),
            0x9 => return Ok(self.v9),
            0xA => return Ok(self.va),
            0xB => return Ok(self.vb),
            0xC => return Ok(self.vc),
            0xD => return Ok(self.vd),
            0xE => return Ok(self.ve),
            0xF => return Ok(self.vf),
            _ => return Err(ChipError::InvalidRegister),
        }
    }
}

pub enum ChipError {
    IOError(std::io::Error),
    RomTooLarge(usize),
    AddressOutofBounds,
    InvalidInstruction,
    InvalidRegister,
    InvalidSpriteSize,
}
impl Display for ChipError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ChipError::IOError(ioerr) => write!(f, "{}", ioerr),
            ChipError::RomTooLarge(diff) => write!(f, "Rom too large by {} bytes", diff),
            ChipError::AddressOutofBounds => write!(f, "Address out of bounds"),
            ChipError::InvalidInstruction => write!(f, "Invalid Instruction was encountered"),
            ChipError::InvalidRegister => write!(f, "Attempt to access invalid register"),
            ChipError::InvalidSpriteSize => {
                write!(f, "The sprite's size was greater than 15 bytes")
            }
        }
    }
}
