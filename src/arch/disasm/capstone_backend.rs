use capstone::prelude::*;
use capstone::Capstone;

pub struct CapstoneDisasm {
  cs: Capstone,
}

impl CapstoneDisasm {
  #[cfg(target_arch = "x86")]
  pub fn new() -> Result<Self, capstone::Error> {
    let cs = Capstone::new()
      .x86()
      .mode(arch::x86::ArchMode::Mode32)
      .detail(true)
      .build()?;
    Ok(Self { cs })
  }

  #[cfg(target_arch = "x86_64")]
  pub fn new() -> Result<Self, capstone::Error> {
    let cs = Capstone::new()
      .x86()
      .mode(arch::x86::ArchMode::Mode64)
      .detail(true)
      .build()?;
    Ok(Self { cs })
  }

  #[cfg(target_arch = "aarch64")]
  pub fn new() -> Result<Self, capstone::Error> {
    let cs = Capstone::new()
      .arm64()
      .mode(arch::arm64::ArchMode::Arm)
      .detail(true)
      .build()?;
    Ok(Self { cs })
  }

  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  pub fn disasm_owned(&self, address: u64, bytes: &[u8]) -> Vec<OwnedInsn> {
    use capstone::arch::x86::{X86Insn, X86InsnGroup, X86OperandType, X86Reg};

    let Ok(instructions) = self.cs.disasm_all(bytes, address) else {
      return Vec::new();
    };

    instructions
      .iter()
      .map(|insn| {
        let detail = self.cs.insn_detail(&insn).ok();

        let is_branch = detail
          .as_ref()
          .map(|d| {
            d.groups()
              .any(|g| g.0 as u32 == X86InsnGroup::X86_GRP_JUMP as u32)
          })
          .unwrap_or(false);

        let is_call = detail
          .as_ref()
          .map(|d| {
            d.groups()
              .any(|g| g.0 as u32 == X86InsnGroup::X86_GRP_CALL as u32)
          })
          .unwrap_or(false);

        let is_return = detail
          .as_ref()
          .map(|d| {
            d.groups()
              .any(|g| g.0 as u32 == X86InsnGroup::X86_GRP_RET as u32)
          })
          .unwrap_or(false);

        let is_unconditional_jump = insn.id().0 as u32 == X86Insn::X86_INS_JMP as u32;

        let is_loop = {
          let id = insn.id().0 as u32;
          id == X86Insn::X86_INS_LOOP as u32
            || id == X86Insn::X86_INS_LOOPE as u32
            || id == X86Insn::X86_INS_LOOPNE as u32
            || id == X86Insn::X86_INS_JECXZ as u32
            || id == X86Insn::X86_INS_JCXZ as u32
            || id == X86Insn::X86_INS_JRCXZ as u32
        };

        let mut branch_target = None;
        let mut has_rip_relative = false;
        let mut rip_relative_target = None;

        if let Some(ref d) = detail {
          let arch_detail = d.arch_detail();
          if let Some(x86_detail) = arch_detail.x86() {
            for op in x86_detail.operands() {
              match op.op_type {
                X86OperandType::Imm(imm) => {
                  if is_branch || is_call {
                    branch_target = Some(imm as u64);
                  }
                },
                X86OperandType::Mem(mem) => {
                  if mem.base().0 as u32 == X86Reg::X86_REG_RIP as u32 {
                    has_rip_relative = true;
                    let disp = mem.disp();
                    let next_ip = insn.address() + insn.len() as u64;
                    rip_relative_target = Some((next_ip as i64 + disp) as u64);
                  }
                },
                _ => {},
              }
            }
          }
        }

        OwnedInsn {
          address: insn.address(),
          bytes: insn.bytes().to_vec(),
          mnemonic: insn.mnemonic().unwrap_or("").to_string(),
          op_str: insn.op_str().unwrap_or("").to_string(),
          is_branch,
          is_call,
          is_return,
          is_unconditional_jump,
          is_loop,
          branch_target,
          has_rip_relative,
          rip_relative_target,
        }
      })
      .collect()
  }

  #[cfg(target_arch = "aarch64")]
  pub fn disasm_owned(&self, address: u64, bytes: &[u8]) -> Vec<OwnedInsn> {
    let Ok(instructions) = self.cs.disasm_all(bytes, address) else {
      return Vec::new();
    };

    instructions
      .iter()
      .map(|insn| {
        let mnemonic = insn.mnemonic().unwrap_or("").to_string();
        let op_str = insn.op_str().unwrap_or("").to_string();

        // Parse branch target from operand string
        let branch_target = if op_str.starts_with("#0x") {
          u64::from_str_radix(op_str.trim_start_matches("#0x"), 16).ok()
        } else if op_str.starts_with("#") {
          op_str
            .trim_start_matches("#")
            .parse::<i64>()
            .ok()
            .map(|offset| (insn.address() as i64 + offset) as u64)
        } else {
          None
        };

        OwnedInsn {
          address: insn.address(),
          bytes: insn.bytes().to_vec(),
          mnemonic,
          op_str,
          branch_target,
        }
      })
      .collect()
  }
}

#[derive(Clone)]
pub struct OwnedInsn {
  address: u64,
  bytes: Vec<u8>,
  mnemonic: String,
  #[cfg_attr(target_arch = "aarch64", allow(dead_code))]
  op_str: String,
  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  is_branch: bool,
  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  is_call: bool,
  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  is_return: bool,
  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  is_unconditional_jump: bool,
  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  is_loop: bool,
  branch_target: Option<u64>,
  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  has_rip_relative: bool,
  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  rip_relative_target: Option<u64>,
}

impl OwnedInsn {
  pub fn len(&self) -> usize {
    self.bytes.len()
  }

  pub fn address(&self) -> u64 {
    self.address
  }

  pub fn bytes(&self) -> &[u8] {
    &self.bytes
  }

  pub fn mnemonic(&self) -> &str {
    &self.mnemonic
  }

  #[cfg_attr(target_arch = "aarch64", allow(dead_code))]
  pub fn op_str(&self) -> &str {
    &self.op_str
  }

  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  pub fn is_branch(&self) -> bool {
    self.is_branch
  }

  #[cfg(target_arch = "aarch64")]
  pub fn is_branch(&self) -> bool {
    let m = &self.mnemonic;
    m.starts_with("b") && m != "bic" && m != "bics" && m != "bfi" && m != "bfm" && m != "bfxil"
  }

  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  pub fn is_call(&self) -> bool {
    self.is_call
  }

  #[cfg(target_arch = "aarch64")]
  pub fn is_call(&self) -> bool {
    self.mnemonic == "bl" || self.mnemonic == "blr"
  }

  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  pub fn is_return(&self) -> bool {
    self.is_return
  }

  #[cfg(target_arch = "aarch64")]
  pub fn is_return(&self) -> bool {
    self.mnemonic == "ret"
  }

  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  pub fn is_unconditional_jump(&self) -> bool {
    self.is_unconditional_jump
  }

  #[cfg(target_arch = "aarch64")]
  pub fn is_unconditional_jump(&self) -> bool {
    self.mnemonic == "b" || self.mnemonic == "br"
  }

  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  pub fn is_loop(&self) -> bool {
    self.is_loop
  }

  #[cfg_attr(target_arch = "aarch64", allow(dead_code))]
  #[cfg(target_arch = "aarch64")]
  pub fn is_loop(&self) -> bool {
    false
  }

  pub fn branch_target(&self) -> Option<u64> {
    self.branch_target
  }

  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  pub fn has_pc_relative_memory(&self) -> bool {
    self.has_rip_relative
  }

  #[cfg_attr(target_arch = "aarch64", allow(dead_code))]
  #[cfg(target_arch = "aarch64")]
  pub fn has_pc_relative_memory(&self) -> bool {
    self.mnemonic == "adr" || self.mnemonic == "adrp" || self.mnemonic.starts_with("ldr")
  }

  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  pub fn pc_relative_target(&self) -> Option<u64> {
    self.rip_relative_target
  }

  #[cfg_attr(target_arch = "aarch64", allow(dead_code))]
  #[cfg(target_arch = "aarch64")]
  pub fn pc_relative_target(&self) -> Option<u64> {
    if self.op_str.contains("#0x") {
      let parts: Vec<&str> = self.op_str.split(',').collect();
      if parts.len() >= 2 {
        let target_str = parts[1].trim().trim_start_matches("#0x");
        return u64::from_str_radix(target_str, 16).ok();
      }
    }
    None
  }

  pub fn next_ip(&self) -> u64 {
    self.address + self.bytes.len() as u64
  }
}
