use super::memory;
use crate::error::{Error, Result};
use crate::{alloc, arch, util};
use std::cell::UnsafeCell;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

/// An architecture-independent implementation of a base detour.
///
/// This class is never instantiated by itself, it merely exposes an API
/// available through it's descendants.
pub struct Detour {
  #[allow(dead_code)]
  relay: Option<alloc::ExecutableMemory>,
  trampoline: alloc::ExecutableMemory,
  patcher: UnsafeCell<arch::Patcher>,
  enabled: AtomicBool,
}

impl Detour {
  pub unsafe fn new(target: *const (), detour: *const ()) -> Result<Self> {
    if target == detour {
      Err(Error::SameAddress)?;
    }

    // Lock this so OS operations are not performed in parallell
    let mut pool = memory::POOL.lock().unwrap();

    if !util::is_executable_address(target)? || !util::is_executable_address(detour)? {
      Err(Error::NotExecutable)?;
    }

    // Create a trampoline generator for the target function
    let margin = arch::meta::prolog_margin(target);
    let trampoline = arch::Trampoline::new(target, margin)?;

    // A relay is used in case a normal branch cannot reach the destination
    let relay = if let Some(emitter) = arch::meta::relay_builder(target, detour)? {
      Some(memory::allocate_pic(&mut pool, &emitter, target)?)
    } else {
      None
    };

    // If a relay is supplied, use it instead of the detour address
    let detour = relay
      .as_ref()
      .map(|code| code.as_ptr() as *const ())
      .unwrap_or(detour);

    Ok(Detour {
      patcher: UnsafeCell::new(arch::Patcher::new(
        target,
        detour,
        trampoline.prolog_size(),
      )?),
      trampoline: memory::allocate_pic(&mut pool, trampoline.emitter(), target)?,
      enabled: AtomicBool::default(),
      relay,
    })
  }

  /// Enables the detour.
  pub unsafe fn enable(&self) -> Result<()> {
    self.toggle(true)
  }

  /// Disables the detour.
  pub unsafe fn disable(&self) -> Result<()> {
    self.toggle(false)
  }

  /// Returns whether the detour is enabled or not.
  pub fn is_enabled(&self) -> bool {
    self.enabled.load(Ordering::SeqCst)
  }

  /// Returns a reference to the generated trampoline.
  pub fn trampoline(&self) -> &() {
    unsafe {
      (self.trampoline.as_ptr() as *const ())
        .as_ref()
        .expect("trampoline should not be null")
    }
  }

  /// Enables or disables the detour.
  unsafe fn toggle(&self, enabled: bool) -> Result<()> {
    let _guard = memory::POOL.lock().unwrap();

    if self.enabled.load(Ordering::SeqCst) == enabled {
      return Ok(());
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
      Self::macos_patch_bytes(&mut (*self.patcher.get()), enabled)?;
    }

    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
      let area = (*self.patcher.get()).area();
      let _handle = region::protect_with_handle(
        area.as_ptr(),
        area.len(),
        region::Protection::READ_WRITE_EXECUTE,
      )?;

      (*self.patcher.get()).toggle(enabled);
    }

    self.enabled.store(enabled, Ordering::SeqCst);
    Ok(())
  }

  #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
  #[cfg_attr(
    all(target_os = "macos", target_arch = "aarch64"),
    link_section = "__TEXT,__detour_patch"
  )]
  #[inline(never)]
  unsafe fn macos_make_writable(addr: usize, len: usize) -> Result<()> {
    const VM_PROT_READ: i32 = 0x01;
    const VM_PROT_WRITE: i32 = 0x02;
    const VM_PROT_COPY: i32 = 0x10;
    const KERN_SUCCESS: i32 = 0;

    extern "C" {
      fn mach_task_self() -> u32;
      fn mach_vm_protect(
        target_task: u32,
        address: u64,
        size: u64,
        set_maximum: i32,
        new_protection: i32,
      ) -> i32;
    }

    let result = mach_vm_protect(
      mach_task_self(),
      addr as u64,
      len as u64,
      0,
      VM_PROT_READ | VM_PROT_WRITE | VM_PROT_COPY,
    );

    if result != KERN_SUCCESS {
      return Err(Error::RegionFailure(region::Error::SystemCall(
        std::io::Error::new(
          std::io::ErrorKind::PermissionDenied,
          format!(
            "mach_vm_protect (make writable) failed: kern_return_t {}",
            result
          ),
        ),
      )));
    }

    Ok(())
  }

  #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
  #[cfg_attr(
    all(target_os = "macos", target_arch = "aarch64"),
    link_section = "__TEXT,__detour_patch"
  )]
  #[inline(never)]
  unsafe fn macos_make_executable(addr: usize, len: usize) -> Result<()> {
    const VM_PROT_READ: i32 = 0x01;
    const VM_PROT_EXECUTE: i32 = 0x04;
    const KERN_SUCCESS: i32 = 0;

    extern "C" {
      fn mach_task_self() -> u32;
      fn mach_vm_protect(
        target_task: u32,
        address: u64,
        size: u64,
        set_maximum: i32,
        new_protection: i32,
      ) -> i32;
    }

    let result = mach_vm_protect(
      mach_task_self(),
      addr as u64,
      len as u64,
      0,
      VM_PROT_READ | VM_PROT_EXECUTE,
    );

    if result != KERN_SUCCESS {
      return Err(Error::RegionFailure(region::Error::SystemCall(
        std::io::Error::new(
          std::io::ErrorKind::PermissionDenied,
          format!(
            "mach_vm_protect (make executable) failed: kern_return_t {}",
            result
          ),
        ),
      )));
    }

    Ok(())
  }

  #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
  #[cfg_attr(
    all(target_os = "macos", target_arch = "aarch64"),
    link_section = "__TEXT,__detour_patch"
  )]
  #[inline(never)]
  unsafe fn macos_flush_icache(ptr: *const u8, len: usize) {
    extern "C" {
      fn sys_icache_invalidate(start: *const u8, size: usize);
    }
    sys_icache_invalidate(ptr, len);
  }

  #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
  #[cfg_attr(
    all(target_os = "macos", target_arch = "aarch64"),
    link_section = "__TEXT,__detour_patch"
  )]
  #[inline(never)]
  unsafe fn macos_patch_bytes(patcher: &mut arch::Patcher, enabled: bool) -> Result<()> {
    let area_ptr = patcher.area().as_ptr();
    let area_len = patcher.area().len();
    let page_size = region::page::size();
    let aligned_addr = (area_ptr as usize) & !(page_size - 1);
    let end_addr = (area_ptr as usize + area_len + page_size - 1) & !(page_size - 1);
    let aligned_len = end_addr - aligned_addr;

    Self::macos_make_writable(aligned_addr, aligned_len)?;
    patcher.toggle(enabled);
    Self::macos_make_executable(aligned_addr, aligned_len)?;
    Self::macos_flush_icache(area_ptr, area_len);
    Ok(())
  }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[repr(align(16384))]
struct DetourPatchSectionAlign(#[allow(dead_code)] [u8; 16384]);

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[cfg_attr(
  all(target_os = "macos", target_arch = "aarch64"),
  link_section = "__TEXT,__detour_patch"
)]
#[used]
static DETOUR_PATCH_SECTION_ALIGN: DetourPatchSectionAlign = DetourPatchSectionAlign([0; 16384]);

impl Drop for Detour {
  /// Disables the detour, if enabled.
  fn drop(&mut self) {
    let did_succeed = unsafe { self.disable() }.is_ok();
    debug_assert!(did_succeed);
  }
}

impl fmt::Debug for Detour {
  /// Output whether the detour is enabled or not.
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(
      f,
      "Detour {{ enabled: {}, trampoline: {:?} }}",
      self.is_enabled(),
      self.trampoline()
    )
  }
}

unsafe impl Send for Detour {}
unsafe impl Sync for Detour {}
