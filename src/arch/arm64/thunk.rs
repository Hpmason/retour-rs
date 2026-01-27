use crate::pic::{FixedThunk, Thunkable};
use generic_array::{typenum, GenericArray};

const NOP: u32 = 0xD503201F;

pub fn nop() -> Box<dyn Thunkable> {
  Box::new(NOP.to_le_bytes().to_vec())
}

pub fn jmp_abs(destination: usize) -> Box<dyn Thunkable> {
  let ldr_x16_pc8: u32 = 0x58000050;
  let br_x16: u32 = 0xD61F0200;

  let mut bytes = Vec::with_capacity(16);
  bytes.extend_from_slice(&ldr_x16_pc8.to_le_bytes());
  bytes.extend_from_slice(&br_x16.to_le_bytes());
  bytes.extend_from_slice(&(destination as u64).to_le_bytes());

  Box::new(bytes)
}

pub fn call_abs(destination: usize) -> Box<dyn Thunkable> {
  let ldr_x16_pc8: u32 = 0x58000050;
  let blr_x16: u32 = 0xD63F0200;

  let mut bytes = Vec::with_capacity(16);
  bytes.extend_from_slice(&ldr_x16_pc8.to_le_bytes());
  bytes.extend_from_slice(&blr_x16.to_le_bytes());
  bytes.extend_from_slice(&(destination as u64).to_le_bytes());

  Box::new(bytes)
}

pub fn jmp_rel(destination: usize) -> Box<dyn Thunkable> {
  Box::new(FixedThunk::<typenum::U4>::new(move |source| {
    let offset = (destination as isize - source as isize) >> 2;

    assert!(
      (-(1 << 25)..(1 << 25)).contains(&offset),
      "Branch offset out of range"
    );

    let imm26 = (offset as u32) & 0x03FF_FFFF;
    let instruction = 0x14000000 | imm26;

    GenericArray::from_array(instruction.to_le_bytes())
  }))
}

#[allow(dead_code)]
pub fn call_rel(destination: usize) -> Box<dyn Thunkable> {
  Box::new(FixedThunk::<typenum::U4>::new(move |source| {
    let offset = (destination as isize - source as isize) >> 2;

    assert!(
      (-(1 << 25)..(1 << 25)).contains(&offset),
      "Branch offset out of range"
    );

    let imm26 = (offset as u32) & 0x03FF_FFFF;
    let instruction = 0x94000000 | imm26;

    GenericArray::from_array(instruction.to_le_bytes())
  }))
}

pub fn jcc_abs(destination: usize, condition: u8) -> Box<dyn Thunkable> {
  let inverted_cond = condition ^ 1;

  let b_cond_skip: u32 = 0x54000000 | (3 << 5) | (inverted_cond as u32);

  let ldr_x16_pc8: u32 = 0x58000050;

  let br_x16: u32 = 0xD61F0200;

  let mut bytes = Vec::with_capacity(20);
  bytes.extend_from_slice(&b_cond_skip.to_le_bytes());
  bytes.extend_from_slice(&ldr_x16_pc8.to_le_bytes());
  bytes.extend_from_slice(&br_x16.to_le_bytes());
  bytes.extend_from_slice(&(destination as u64).to_le_bytes());

  Box::new(bytes)
}

pub fn jmp(destination: usize) -> Box<dyn Thunkable> {
  jmp_abs(destination)
}

pub fn call(destination: usize) -> Box<dyn Thunkable> {
  call_abs(destination)
}

pub fn jcc(destination: usize, condition: u8) -> Box<dyn Thunkable> {
  jcc_abs(destination, condition)
}

#[allow(dead_code)]
pub mod condition {
  pub const EQ: u8 = 0x0;
  pub const NE: u8 = 0x1;
  pub const CS: u8 = 0x2;
  pub const HS: u8 = 0x2;
  pub const CC: u8 = 0x3;
  pub const LO: u8 = 0x3;
  pub const MI: u8 = 0x4;
  pub const PL: u8 = 0x5;
  pub const VS: u8 = 0x6;
  pub const VC: u8 = 0x7;
  pub const HI: u8 = 0x8;
  pub const LS: u8 = 0x9;
  pub const GE: u8 = 0xA;
  pub const LT: u8 = 0xB;
  pub const GT: u8 = 0xC;
  pub const LE: u8 = 0xD;
  pub const AL: u8 = 0xE;
  pub const NV: u8 = 0xF;
}
