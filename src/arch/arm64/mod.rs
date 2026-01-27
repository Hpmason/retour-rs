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
  use std::mem;

  type CRet = unsafe extern "C" fn() -> i32;

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

  unsafe extern "C" fn ret10() -> i32 {
    10
  }
}
