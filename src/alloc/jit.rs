//! JIT memory protection helpers for macOS ARM64.

pub struct JitWriteGuard {
  #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
  _private: (),
}

impl JitWriteGuard {
  #[inline]
  pub fn new() -> Self {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
      unsafe {
        pthread_jit_write_protect_np(0);
      }
    }
    Self {
      #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
      _private: (),
    }
  }
}

impl Default for JitWriteGuard {
  fn default() -> Self {
    Self::new()
  }
}

impl Drop for JitWriteGuard {
  #[inline]
  fn drop(&mut self) {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
      unsafe {
        pthread_jit_write_protect_np(1);
      }
    }
  }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
extern "C" {
  fn pthread_jit_write_protect_np(enabled: i32);
}

#[inline]
pub fn with_write_access<F, R>(f: F) -> R
where
  F: FnOnce() -> R,
{
  let _guard = JitWriteGuard::new();
  f()
}

#[inline]
#[allow(dead_code)]
pub const fn requires_jit_protection() -> bool {
  cfg!(all(target_os = "macos", target_arch = "aarch64"))
}
