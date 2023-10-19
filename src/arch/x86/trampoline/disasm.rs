//! The underlying disassembler should be opaque to the outside.
use iced_x86::{Instruction, Mnemonic, OpKind, Register};

pub trait InstructionExt {
  /// Returns the instructions relative branch offset, if applicable.
  fn relative_branch_target(&self) -> Option<u64>;
  /// Returns the instructions RIP operand displacement if applicable.
  fn rip_operand_target(&self) -> Option<u64>;
  /// Returns true if this instruction any type of a loop.
  fn is_loop(&self) -> bool;
  /// Returns true if this instruction is an unconditional jump.
  fn is_unconditional_jump(&self) -> bool;
  /// Returns true if this instruction is a function call.
  fn is_call(&self) -> bool;
  /// Returns true if this instruction is a return.
  fn is_return(&self) -> bool;
}

impl InstructionExt for Instruction {
  /// Returns the instructions relative branch offset, if applicable.
  fn relative_branch_target(&self) -> Option<u64> {
    use OpKind::*;
    match self.op0_kind() {
      NearBranch16 | NearBranch32 | NearBranch64 => Some(self.near_branch_target()),
      _ => None,
    }
  }

  /// Returns the instructions RIP operand displacement if applicable.
  fn rip_operand_target(&self) -> Option<u64> {
    self
      .op_kinds()
      .into_iter()
      .find(|op| *op == OpKind::Memory && self.memory_base() == Register::RIP)
      .map(|_| self.memory_displacement64())
  }

  /// Returns true if this instruction any type of a loop.
  fn is_loop(&self) -> bool {
    use Mnemonic::*;
    matches!(self.mnemonic(), Loop | Loope | Loopne | Jecxz | Jcxz)
  }

  /// Returns true if this instruction is an unconditional jump.
  fn is_unconditional_jump(&self) -> bool {
    self.mnemonic() == Mnemonic::Jmp
  }

  /// Returns true if this instruction is a function call.
  fn is_call(&self) -> bool {
    self.mnemonic() == Mnemonic::Call
  }

  /// Returns true if this instruction is a return.
  fn is_return(&self) -> bool {
    self.mnemonic() == Mnemonic::Ret
  }
}
