use crate::arch::disasm::{CapstoneDisasm, OwnedInsn};
use crate::arch::x86::thunk;
use crate::error::{Error, Result};
use crate::pic;
use std::{mem, slice};

/// A trampoline generator (x86/x64).
pub struct Trampoline {
  emitter: pic::CodeEmitter,
  prolog_size: usize,
}

impl Trampoline {
  /// Constructs a new trampoline for an address.
  pub unsafe fn new(target: *const (), margin: usize) -> Result<Trampoline> {
    Builder::new(target, margin).build()
  }

  /// Returns a reference to the trampoline's code emitter.
  pub fn emitter(&self) -> &pic::CodeEmitter {
    &self.emitter
  }

  /// Returns the size of the prolog (i.e the amount of disassembled bytes).
  pub fn prolog_size(&self) -> usize {
    self.prolog_size
  }
}

/// A trampoline builder.
struct Builder {
  /// Target destination for a potential internal branch.
  branch_address: Option<usize>,
  /// Total amount of bytes disassembled.
  total_bytes_disassembled: usize,
  /// The preferred minimum amount of bytes disassembled.
  margin: usize,
  /// Whether disassembling has finished or not.
  finished: bool,
  /// The target the trampoline is adapted for.
  target: *const (),
}

impl Builder {
  /// Returns a trampoline builder.
  pub fn new(target: *const (), margin: usize) -> Self {
    Builder {
      branch_address: None,
      total_bytes_disassembled: 0,
      finished: false,
      target,
      margin,
    }
  }

  /// Creates a trampoline with the supplied settings.
  ///
  /// # Safety
  ///
  /// target..target+margin+15 must be valid to read as a u8 slice or behavior
  /// may be undefined
  pub unsafe fn build(mut self) -> Result<Trampoline> {
    let mut emitter = pic::CodeEmitter::new();

    // 15 = max size of x64 instruction
    // safety: we don't know the end address of a function so this could be too far
    // if the function is right at the end of the code section.
    let target: *const u8 = self.target.cast();
    // Volatile read to prevent the compiler from optimizing away the pointer
    let target = unsafe { std::ptr::read_volatile(&target) };
    let slice = unsafe { slice::from_raw_parts(target, self.margin + 15) };

    let disasm = CapstoneDisasm::new().map_err(|_| Error::InvalidCode)?;
    let instructions = disasm.disasm_owned(self.target as u64, slice);

    for instruction in instructions {
      if instruction.len() == 0 {
        break;
      }
      self.total_bytes_disassembled += instruction.len();
      let thunk = self.process_instruction(&instruction)?;

      // If the trampoline displacement is larger than the target
      // function, all instructions will be displaced, and if there is
      // internal branching, it will end up at the wrong instructions.
      if self.is_instruction_in_branch(&instruction) && instruction.len() != thunk.len() {
        Err(Error::UnsupportedInstruction)?;
      } else {
        emitter.add_thunk(thunk);
      }

      // Determine whether enough bytes for the margin has been disassembled
      if self.total_bytes_disassembled >= self.margin && !self.finished {
        // Add a jump to the first instruction after the prolog
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

  /// Returns an instruction after analysing and potentially modifies it.
  unsafe fn process_instruction(
    &mut self,
    instruction: &OwnedInsn,
  ) -> Result<Box<dyn pic::Thunkable>> {
    if let Some(target) = instruction.pc_relative_target() {
      if instruction.has_pc_relative_memory() {
        return self.handle_rip_relative_instruction(instruction, target as usize);
      }
    }

    if let Some(target) = instruction.branch_target() {
      if instruction.is_branch() || instruction.is_call() {
        return self.handle_relative_branch(instruction, target as usize);
      }
    }

    if instruction.is_return() {
      // In case the operand is not placed in a branch, the function
      // returns unconditionally (i.e it terminates here).
      self.finished = !self.is_instruction_in_branch(instruction);
    }

    // The instruction does not use any position-dependant operands,
    // therefore the bytes can be copied directly from source.
    Ok(Box::new(instruction.bytes().to_vec()))
  }

  /// Adjusts the offsets for RIP relative operands. They are only available
  /// in x64 processes. The operands offsets needs to be adjusted for their
  /// new position. An example would be:
  ///
  /// ```asm
  /// mov eax, [rip+0x10]   ; the displacement before relocation
  /// mov eax, [rip+0x4892] ; theoretical adjustment after relocation
  /// ```
  unsafe fn handle_rip_relative_instruction(
    &mut self,
    instruction: &OwnedInsn,
    target: usize,
  ) -> Result<Box<dyn pic::Thunkable>> {
    let displacement = target
      .wrapping_sub(instruction.address() as usize)
      .wrapping_sub(instruction.len()) as isize;
    // If the instruction is an unconditional jump, processing stops here
    self.finished = instruction.is_unconditional_jump();

    // Nothing should be done if `displacement` is within the prolog.
    if (-(self.total_bytes_disassembled as isize)..0).contains(&displacement) {
      return Ok(Box::new(instruction.bytes().to_vec()));
    }

    // These need to be captured by the closure
    let instruction_address = instruction.address() as isize;
    let instruction_bytes = instruction.bytes().to_vec();
    let instruction_len = instruction.len();

    let immediate_size = Self::estimate_immediate_size(instruction);

    Ok(Box::new(pic::UnsafeThunk::new(
      move |offset| {
        let mut bytes = instruction_bytes.clone();

        // Calculate the new relative displacement for the operand. The
        // instruction is relative so the offset (i.e where the trampoline is
        // allocated), must be within a range of +/- 2GB.
        let adjusted_displacement = instruction_address
          .wrapping_sub(offset as isize)
          .wrapping_add(displacement);
        assert!(crate::arch::is_within_range(adjusted_displacement));

        // The displacement value is placed at (instruction - disp32 - imm)
        let index = instruction_bytes.len() - mem::size_of::<u32>() - immediate_size;

        // Write the adjusted displacement offset to the operand
        let as_bytes: [u8; 4] = (adjusted_displacement as u32).to_ne_bytes();
        bytes[index..index + as_bytes.len()].copy_from_slice(&as_bytes);
        bytes
      },
      instruction_len,
    )))
  }

  /// Estimate the immediate operand size from instruction bytes.
  fn estimate_immediate_size(instruction: &OwnedInsn) -> usize {
    let op_str = instruction.op_str();

    if op_str.contains(", 0x") || op_str.contains(", -0x") {
      if let Some(imm_start) = op_str.rfind(", ") {
        let imm_str = &op_str[imm_start + 2..];
        if imm_str.starts_with("0x") || imm_str.starts_with("-0x") {
          let hex_str = imm_str.trim_start_matches('-').trim_start_matches("0x");
          if hex_str.len() > 8 {
            return 8;
          } else if hex_str.len() > 4 {
            return 4;
          } else if hex_str.len() > 2 {
            return 2;
          } else {
            return 1;
          }
        }
      }
    }
    0
  }

  /// Processes relative branches (e.g `call`, `loop`, `jne`).
  unsafe fn handle_relative_branch(
    &mut self,
    instruction: &OwnedInsn,
    destination_address_abs: usize,
  ) -> Result<Box<dyn pic::Thunkable>> {
    if instruction.is_call() {
      // Calls are not an issue since they return to the original address
      return Ok(thunk::call(destination_address_abs));
    }

    let prolog_range = (self.target as usize)..(self.target as usize + self.margin);

    // If the relative jump is internal, and short enough to
    // fit within the copied function prolog (i.e `margin`),
    // the jump instruction can be copied indiscriminately.
    if prolog_range.contains(&destination_address_abs) {
      // Keep track of the jump's destination address
      self.branch_address = Some(destination_address_abs);
      Ok(Box::new(instruction.bytes().to_vec()))
    } else if instruction.is_loop() {
      // Loops (e.g 'loopnz', 'jecxz') to the outside are not supported
      Err(Error::UnsupportedInstruction)
    } else if instruction.is_unconditional_jump() {
      // If the function is not in a branch, and it unconditionally jumps
      // a distance larger than the prolog, it's the same as if it terminates.
      self.finished = !self.is_instruction_in_branch(instruction);
      Ok(thunk::jmp(destination_address_abs))
    } else {
      // Conditional jumps (Jcc)
      // To extract the condition, the primary opcode is required. Short
      // jumps are only one byte, but long jccs are prefixed with 0x0F.
      let instruction_bytes = instruction.bytes();
      let primary_opcode = instruction_bytes
        .iter()
        .find(|op| **op != 0x0F)
        .expect("retrieving conditional jump primary op code");

      // Extract the condition (i.e 0x74 is [jz rel8] ⟶ 0x74 & 0x0F == 4)
      let condition = primary_opcode & 0x0F;
      Ok(thunk::jcc(destination_address_abs, condition))
    }
  }

  /// Returns whether the current instruction is inside a branch or not.
  fn is_instruction_in_branch(&self, instruction: &OwnedInsn) -> bool {
    self
      .branch_address
      .map_or(false, |offset| instruction.address() < offset as u64)
  }
}
