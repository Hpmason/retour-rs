pub use self::patcher::Patcher;
pub use self::trampoline::Trampoline;

pub mod meta;
pub mod thunk;

mod patcher;
mod trampoline;

#[cfg(all(feature = "nightly", test, target_arch = "aarch64"))]
mod tests {
  use crate::error::Result;
  use crate::RawDetour;
  use std::arch::naked_asm;
  use std::mem;

  #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
  #[repr(align(16384))]
  struct DetourTestSectionAlign(#[allow(dead_code)] [u8; 16384]);

  #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
  #[link_section = "__TEXT,__detour_test"]
  #[used]
  static DETOUR_TEST_SECTION_ALIGN: DetourTestSectionAlign = DetourTestSectionAlign([0; 16384]);

  /// Default test case function definition.
  type CRet = unsafe extern "C" fn() -> i32;

  /// Detours a C function returning an integer, and asserts its return value.
  #[inline(never)]
  unsafe fn detour_test(target: CRet, result: i32) -> Result<()> {
    let hook = RawDetour::new(target as *const (), ret10 as *const ())?;

    assert_eq!(target(), result);
    hook.enable()?;
    {
      assert_eq!(target(), 10);
      let original: CRet = mem::transmute(hook.trampoline());
      assert_eq!(original(), result);
    }
    hook.disable()?;
    assert_eq!(target(), result);
    Ok(())
  }

  #[test]
  fn detour_simple() -> Result<()> {
    #[inline(never)]
    #[cfg_attr(all(target_os = "macos", target_arch = "aarch64"), link_section = "__TEXT,__detour_test")]
    unsafe extern "C" fn ret5() -> i32 {
      let result = 5i32;
      std::hint::black_box(result);
      std::hint::black_box(result);
      std::hint::black_box(result);
      std::hint::black_box(result);
      result
    }

    unsafe { detour_test(mem::transmute(ret5 as *const () as usize), 5) }
  }

  #[test]
  fn detour_conditional_branch() -> Result<()> {
    // Tests a function with a conditional branch (b.eq) in the prolog
    #[unsafe(naked)]
    #[cfg_attr(all(target_os = "macos", target_arch = "aarch64"), link_section = "__TEXT,__detour_test")]
    unsafe extern "C" fn branch_ret5() -> i32 {
      naked_asm!(
        "
            mov w0, #0
            cmp w0, #0
            b.eq 1f
            mov w0, #2
            b 2f
          1:
            mov w0, #5
          2:
            ret",
      );
    }

    unsafe { detour_test(mem::transmute(branch_ret5 as *const () as usize), 5) }
  }

  #[test]
  fn detour_unconditional_branch() -> Result<()> {
    // Tests a function with an unconditional branch (b) in the prolog
    #[unsafe(naked)]
    #[cfg_attr(all(target_os = "macos", target_arch = "aarch64"), link_section = "__TEXT,__detour_test")]
    unsafe extern "C" fn unconditional_branch_ret7() -> i32 {
      naked_asm!(
        "
            mov w0, #7
            nop
            nop
            b 1f
            mov w0, #99
          1:
            ret",
      );
    }

    unsafe { detour_test(mem::transmute(unconditional_branch_ret7 as *const () as usize), 7) }
  }

  #[test]
  fn detour_adr_relative() -> Result<()> {
    // Tests a function using ADR instruction for PC-relative addressing
    #[unsafe(naked)]
    #[cfg_attr(all(target_os = "macos", target_arch = "aarch64"), link_section = "__TEXT,__detour_test")]
    unsafe extern "C" fn adr_relative_ret42() -> i32 {
      naked_asm!(
        "
            adr x0, 1f
            ldrb w0, [x0]
            nop
            nop
            nop
            ret
          1:
            .byte 42",
      );
    }

    unsafe { detour_test(mem::transmute(adr_relative_ret42 as *const () as usize), 42) }
  }

  #[test]
  fn detour_ldr_literal_64bit() -> Result<()> {
    // Tests a function using 64-bit LDR literal (PC-relative load)
    // Note: 32-bit LDR literal (ldr wN) is not currently supported
    #[unsafe(naked)]
    #[cfg_attr(all(target_os = "macos", target_arch = "aarch64"), link_section = "__TEXT,__detour_test")]
    unsafe extern "C" fn ldr_literal_ret123() -> i32 {
      naked_asm!(
        "
            ldr x0, 1f
            nop
            nop
            nop
            ret
            .p2align 3
          1:
            .quad 123",
      );
    }

    unsafe { detour_test(mem::transmute(ldr_literal_ret123 as *const () as usize), 123) }
  }

  #[test]
  fn detour_padding_after() -> Result<()> {
    // Tests a function with NOP padding after the return
    #[unsafe(naked)]
    #[cfg_attr(all(target_os = "macos", target_arch = "aarch64"), link_section = "__TEXT,__detour_test")]
    unsafe extern "C" fn padding_after_ret3() -> i32 {
      naked_asm!(
        "
            mov w0, #3
            ret
            nop
            nop
            nop
            nop",
      );
    }

    unsafe { detour_test(mem::transmute(padding_after_ret3 as *const () as usize), 3) }
  }

  #[test]
  fn detour_hotpatch() -> Result<()> {
    // Tests a function with NOP padding before the actual code (hotpatch style)
    #[unsafe(naked)]
    #[cfg_attr(all(target_os = "macos", target_arch = "aarch64"), link_section = "__TEXT,__detour_test")]
    unsafe extern "C" fn hotpatch_ret0() -> i32 {
      naked_asm!(
        "
            nop
            nop
            nop
            nop
            mov w0, #0
            nop
            nop
            nop
            ret",
      );
    }

    // Detour starting after the NOPs (at offset 16)
    unsafe { detour_test(mem::transmute(hotpatch_ret0 as *const () as usize + 16), 0) }
  }

  /// Default detour target.
  #[cfg_attr(all(target_os = "macos", target_arch = "aarch64"), link_section = "__TEXT,__detour_test")]
  unsafe extern "C" fn ret10() -> i32 {
    10
  }
}
