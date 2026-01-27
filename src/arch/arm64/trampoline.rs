use crate::arch::arm64::thunk;
use crate::arch::disasm::{CapstoneDisasm, OwnedInsn};
use crate::error::{Error, Result};
use crate::pic;
use std::slice;

pub struct Trampoline {
  emitter: pic::CodeEmitter,
  prolog_size: usize,
}

impl Trampoline {
  pub unsafe fn new(target: *const (), margin: usize) -> Result<Trampoline> {
    Builder::new(target, margin).build()
  }

  pub fn emitter(&self) -> &pic::CodeEmitter {
    &self.emitter
  }

  pub fn prolog_size(&self) -> usize {
    self.prolog_size
  }
}

struct Builder {
  branch_address: Option<usize>,
  total_bytes_disassembled: usize,
  margin: usize,
  finished: bool,
  target: *const (),
}

impl Builder {
  pub fn new(target: *const (), margin: usize) -> Self {
    Builder {
      branch_address: None,
      total_bytes_disassembled: 0,
      finished: false,
      target,
      margin,
    }
  }

  pub unsafe fn build(mut self) -> Result<Trampoline> {
    let mut emitter = pic::CodeEmitter::new();

    let target: *const u8 = self.target.cast();
    // Volatile read to prevent the compiler from optimizing away the pointer
    let target = unsafe { std::ptr::read_volatile(&target) };
    let slice = unsafe { slice::from_raw_parts(target, self.margin + 16) };

    let disasm = CapstoneDisasm::new().map_err(|_| Error::InvalidCode)?;
    let instructions = disasm.disasm_owned(self.target as u64, slice);

    for instruction in instructions {
      if instruction.len() == 0 {
        break;
      }

      self.total_bytes_disassembled += instruction.len();
      let thunk = self.process_instruction(&instruction)?;

      if self.is_instruction_in_branch(&instruction) && instruction.len() != thunk.len() {
        return Err(Error::UnsupportedInstruction);
      } else {
        emitter.add_thunk(thunk);
      }

      if self.total_bytes_disassembled >= self.margin && !self.finished {
        emitter.add_thunk(thunk::jmp(instruction.next_ip() as usize));
        self.finished = true;
      }

      if self.finished {
        break;
      }
    }

    Ok(Trampoline {
      prolog_size: self.total_bytes_disassembled,
      emitter,
    })
  }

  unsafe fn process_instruction(
    &mut self,
    instruction: &OwnedInsn,
  ) -> Result<Box<dyn pic::Thunkable>> {
    let mnemonic = instruction.mnemonic();

    if Self::is_pc_relative_instruction(mnemonic) {
      return self.handle_pc_relative_instruction(instruction);
    }

    if instruction.is_branch() || instruction.is_call() {
      if let Some(target) = instruction.branch_target() {
        return self.handle_relative_branch(instruction, target as usize);
      }
    }

    if instruction.is_return() {
      self.finished = !self.is_instruction_in_branch(instruction);
    }

    Ok(Box::new(instruction.bytes().to_vec()))
  }

  fn is_pc_relative_instruction(mnemonic: &str) -> bool {
    matches!(mnemonic, "adr" | "adrp" | "ldr" | "ldrsw" | "prfm")
  }

  unsafe fn handle_pc_relative_instruction(
    &mut self,
    instruction: &OwnedInsn,
  ) -> Result<Box<dyn pic::Thunkable>> {
    let mnemonic = instruction.mnemonic();
    let bytes = instruction.bytes();

    match mnemonic {
      "adr" | "adrp" => {
        let insn_word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let rd = (insn_word & 0x1F) as u8;

        if let Some(target_addr) = Self::calculate_pc_rel_target(instruction) {
          return Ok(self.emit_load_absolute(rd, target_addr));
        }

        Ok(Box::new(bytes.to_vec()))
      },
      "ldr" | "ldrsw" => {
        let insn_word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

        let opc = (insn_word >> 30) & 0x3;
        let v = (insn_word >> 26) & 0x1;

        if opc == 0b01 || opc == 0b11 || v == 1 {
          if let Some(target_addr) = Self::calculate_pc_rel_target(instruction) {
            let rd = (insn_word & 0x1F) as u8;
            return Ok(self.emit_ldr_from_absolute(rd, target_addr, opc == 0b11));
          }
        }

        Ok(Box::new(bytes.to_vec()))
      },
      _ => Ok(Box::new(bytes.to_vec())),
    }
  }

  fn calculate_pc_rel_target(instruction: &OwnedInsn) -> Option<u64> {
    let bytes = instruction.bytes();
    let insn_word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let pc = instruction.address();
    let mnemonic = instruction.mnemonic();

    match mnemonic {
      "adr" => {
        let immlo = (insn_word >> 29) & 0x3;
        let immhi = (insn_word >> 5) & 0x7FFFF;
        let imm = ((immhi << 2) | immlo) as i32;
        let imm = if imm & (1 << 20) != 0 {
          imm | !0x1FFFFF
        } else {
          imm
        };
        Some((pc as i64 + imm as i64) as u64)
      },
      "adrp" => {
        let immlo = (insn_word >> 29) & 0x3;
        let immhi = (insn_word >> 5) & 0x7FFFF;
        let imm = ((immhi << 2) | immlo) as i32;
        let imm = if imm & (1 << 20) != 0 {
          imm | !0x1FFFFF
        } else {
          imm
        };
        let page_offset = (imm as i64) << 12;
        let page_base = (pc as i64) & !0xFFF;
        Some((page_base + page_offset) as u64)
      },
      "ldr" | "ldrsw" => {
        let imm19 = (insn_word >> 5) & 0x7FFFF;
        let imm = imm19 as i32;
        let imm = if imm & (1 << 18) != 0 {
          imm | !0x7FFFF
        } else {
          imm
        };
        Some((pc as i64 + (imm as i64) * 4) as u64)
      },
      _ => None,
    }
  }

  fn emit_load_absolute(&self, rd: u8, address: u64) -> Box<dyn pic::Thunkable> {
    let mut bytes = Vec::with_capacity(16);

    let imm0 = (address & 0xFFFF) as u32;
    let imm1 = ((address >> 16) & 0xFFFF) as u32;
    let imm2 = ((address >> 32) & 0xFFFF) as u32;
    let imm3 = ((address >> 48) & 0xFFFF) as u32;

    let movz = 0xD2800000 | (imm0 << 5) | (rd as u32);
    bytes.extend_from_slice(&movz.to_le_bytes());

    if imm1 != 0 || imm2 != 0 || imm3 != 0 {
      let movk1 = 0xF2A00000 | (imm1 << 5) | (rd as u32);
      bytes.extend_from_slice(&movk1.to_le_bytes());
    }

    if imm2 != 0 || imm3 != 0 {
      let movk2 = 0xF2C00000 | (imm2 << 5) | (rd as u32);
      bytes.extend_from_slice(&movk2.to_le_bytes());
    }

    if imm3 != 0 {
      let movk3 = 0xF2E00000 | (imm3 << 5) | (rd as u32);
      bytes.extend_from_slice(&movk3.to_le_bytes());
    }

    Box::new(bytes)
  }

  fn emit_ldr_from_absolute(
    &self,
    rd: u8,
    address: u64,
    is_64bit: bool,
  ) -> Box<dyn pic::Thunkable> {
    let mut bytes = Vec::with_capacity(24);

    let imm0 = (address & 0xFFFF) as u32;
    let imm1 = ((address >> 16) & 0xFFFF) as u32;
    let imm2 = ((address >> 32) & 0xFFFF) as u32;
    let imm3 = ((address >> 48) & 0xFFFF) as u32;

    let movz = 0xD2800000 | (imm0 << 5) | 16;
    bytes.extend_from_slice(&movz.to_le_bytes());

    if imm1 != 0 || imm2 != 0 || imm3 != 0 {
      let movk1 = 0xF2A00000 | (imm1 << 5) | 16;
      bytes.extend_from_slice(&movk1.to_le_bytes());
    }

    if imm2 != 0 || imm3 != 0 {
      let movk2 = 0xF2C00000 | (imm2 << 5) | 16;
      bytes.extend_from_slice(&movk2.to_le_bytes());
    }

    if imm3 != 0 {
      let movk3 = 0xF2E00000 | (imm3 << 5) | 16;
      bytes.extend_from_slice(&movk3.to_le_bytes());
    }

    let ldr = if is_64bit {
      0xF9400200 | (rd as u32)
    } else {
      0xB9400200 | (rd as u32)
    };
    bytes.extend_from_slice(&ldr.to_le_bytes());

    Box::new(bytes)
  }

  unsafe fn handle_relative_branch(
    &mut self,
    instruction: &OwnedInsn,
    destination_address_abs: usize,
  ) -> Result<Box<dyn pic::Thunkable>> {
    if instruction.is_call() {
      return Ok(thunk::call(destination_address_abs));
    }

    let prolog_range = (self.target as usize)..(self.target as usize + self.margin);

    if prolog_range.contains(&destination_address_abs) {
      self.branch_address = Some(destination_address_abs);
      Ok(Box::new(instruction.bytes().to_vec()))
    } else if instruction.is_unconditional_jump() {
      self.finished = !self.is_instruction_in_branch(instruction);
      Ok(thunk::jmp(destination_address_abs))
    } else {
      let bytes = instruction.bytes();
      let insn_word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
      let condition = (insn_word & 0xF) as u8;
      Ok(thunk::jcc(destination_address_abs, condition))
    }
  }

  fn is_instruction_in_branch(&self, instruction: &OwnedInsn) -> bool {
    self
      .branch_address
      .map_or(false, |offset| instruction.address() < offset as u64)
  }
}
