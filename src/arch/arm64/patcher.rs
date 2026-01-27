use super::thunk;
use crate::error::{Error, Result};
use crate::pic;
use std::slice;

pub struct Patcher {
  patch_area: &'static mut [u8],
  original_prolog: Vec<u8>,
  detour_prolog: Vec<u8>,
}

impl Patcher {
  pub unsafe fn new(target: *const (), detour: *const (), prolog_size: usize) -> Result<Patcher> {
    let patch_area = Self::patch_area(target, prolog_size)?;
    let emitter = Self::hook_template(target, detour, patch_area);

    let patch_address = patch_area.as_ptr() as *const ();
    let original_prolog = patch_area.to_vec();

    Ok(Patcher {
      detour_prolog: emitter.emit(patch_address),
      original_prolog,
      patch_area,
    })
  }

  pub fn area(&self) -> &[u8] {
    self.patch_area
  }

  pub unsafe fn toggle(&mut self, enable: bool) {
    self.patch_area.copy_from_slice(if enable {
      &self.detour_prolog
    } else {
      &self.original_prolog
    });

    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    Self::flush_icache(self.patch_area.as_ptr(), self.patch_area.len());
  }

  unsafe fn patch_area(target: *const (), prolog_size: usize) -> Result<&'static mut [u8]> {
    const ABS_JUMP_SIZE: usize = 16;

    if !Self::is_patchable(target, prolog_size, ABS_JUMP_SIZE) {
      return Err(Error::NoPatchArea);
    }

    Ok(slice::from_raw_parts_mut(target as *mut u8, ABS_JUMP_SIZE))
  }

  fn hook_template(target: *const (), detour: *const (), patch_area: &[u8]) -> pic::CodeEmitter {
    let mut emitter = pic::CodeEmitter::new();

    let displacement = (detour as isize).wrapping_sub(target as isize);
    if crate::arch::is_within_range(displacement) {
      emitter.add_thunk(thunk::jmp_rel(detour as usize));
    } else {
      emitter.add_thunk(thunk::jmp_abs(detour as usize));
    }

    while emitter.len() < patch_area.len() {
      emitter.add_thunk(thunk::nop());
    }

    emitter
  }

  unsafe fn is_patchable(target: *const (), prolog_size: usize, patch_size: usize) -> bool {
    if prolog_size >= patch_size {
      return true;
    }

    let slice = slice::from_raw_parts(
      (target as usize + prolog_size) as *const u8,
      patch_size - prolog_size,
    );

    Self::is_code_padding(slice)
  }

  fn is_code_padding(buffer: &[u8]) -> bool {
    const ARM64_NOP: [u8; 4] = [0x1F, 0x20, 0x03, 0xD5];

    if buffer.len() % 4 != 0 {
      return false;
    }

    buffer
      .chunks(4)
      .all(|chunk| chunk == ARM64_NOP || chunk.iter().all(|&b| b == 0x00))
  }

  #[cfg(target_os = "linux")]
  fn flush_icache(ptr: *const u8, len: usize) {
    // The libc crate doesn't export __clear_cache on all platforms.
    // Don't know why, but let's declare it an extern function.
    extern "C" {
      fn __clear_cache(start: *mut libc::c_void, end: *mut libc::c_void);
    }
    unsafe {
      __clear_cache(
        ptr as *mut libc::c_void,
        (ptr as usize + len) as *mut libc::c_void,
      );
    }
  }

  #[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
  fn flush_icache(ptr: *const u8, len: usize) {
    unsafe {
      extern "C" {
        fn sys_icache_invalidate(start: *const u8, size: usize);
      }
      sys_icache_invalidate(ptr, len);
    }
  }

  #[cfg(target_os = "windows")]
  fn flush_icache(ptr: *const u8, len: usize) {
    unsafe {
      extern "system" {
        fn FlushInstructionCache(
          hProcess: *mut std::ffi::c_void,
          lpBaseAddress: *const u8,
          dwSize: usize,
        ) -> i32;
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
      }
      FlushInstructionCache(GetCurrentProcess(), ptr, len);
    }
  }

  #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
  fn flush_icache(_ptr: *const u8, _len: usize) {}
}
