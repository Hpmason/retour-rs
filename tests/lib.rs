use retour::Result;
use std::mem;

type FnAdd = extern "C" fn(i32, i32) -> i32;

#[inline(never)]
extern "C" fn sub_detour(x: i32, y: i32) -> i32 {
  unsafe { std::ptr::read_volatile(&x as *const i32) - y }
}


mod raw {
  use super::*;
  use retour::RawDetour;

  #[test]
  fn test() -> Result<()> {
    #[inline(never)]
    extern "C" fn add(x: i32, y: i32) -> i32 {
      unsafe { std::ptr::read_volatile(&x as *const i32) + y }
    }

    unsafe {
      let hook = RawDetour::new(add as *const (), sub_detour as *const ())
        .expect("target or source is not usable for detouring");

      assert_eq!(add(10, 5), 15);
      assert!(!hook.is_enabled());

      hook.enable()?;
      {
        assert!(hook.is_enabled());

        // The `add` function is hooked, but can be called using the trampoline
        let trampoline: FnAdd = mem::transmute(hook.trampoline());

        // Call the original function
        assert_eq!(trampoline(10, 5), 15);

        // Call the hooked function (i.e `add → sub_detour`)
        assert_eq!(add(10, 5), 5);
      }
      hook.disable()?;

      // With the hook disabled, the function is restored
      assert!(!hook.is_enabled());
      assert_eq!(add(10, 5), 15);
    }
    Ok(())
  }
}

mod generic {
  use super::*;
  use retour::GenericDetour;

  #[test]
  fn test() -> Result<()> {
    #[inline(never)]
    extern "C" fn add(x: i32, y: i32) -> i32 {
      unsafe { std::ptr::read_volatile(&x as *const i32) + y }
    }

    unsafe {
      let hook = GenericDetour::<FnAdd>::new(add, sub_detour)
        .expect("target or source is not usable for detouring");

      assert_eq!(add(10, 5), 15);
      assert_eq!(hook.call(10, 5), 15);
      hook.enable()?;
      {
        assert_eq!(hook.call(10, 5), 15);
        assert_eq!(add(10, 5), 5);
      }
      hook.disable()?;
      assert_eq!(hook.call(10, 5), 15);
      assert_eq!(add(10, 5), 15);
    }
    Ok(())
  }
}

#[cfg(feature = "static-detour")]
mod statik {
  use super::*;
  use retour::static_detour;

  #[inline(never)]
  unsafe extern "C" fn add(x: i32, y: i32) -> i32 {
    std::ptr::read_volatile(&x as *const i32) + y
  }

  static_detour! {
    #[doc="Test with attributes"]
    pub static DetourAdd: unsafe extern "C" fn(i32, i32) -> i32;
  }

  #[test]
  fn test() -> Result<()> {
    unsafe {
      DetourAdd.initialize(add, |x, y| x - y)?;

      assert_eq!(add(10, 5), 15);
      assert_eq!(DetourAdd.is_enabled(), false);

      DetourAdd.enable()?;
      {
        assert!(DetourAdd.is_enabled());
        assert_eq!(DetourAdd.call(10, 5), 15);
        assert_eq!(add(10, 5), 5);
      }
      DetourAdd.disable()?;

      assert_eq!(DetourAdd.is_enabled(), false);
      assert_eq!(DetourAdd.call(10, 5), 15);
      assert_eq!(add(10, 5), 15);
    }
    Ok(())
  }
}

#[cfg(feature = "28-args")]
mod args_28 {
  use super::*;
  use retour::GenericDetour;


  type I = i32;
  type BigFn = fn(I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I);

  fn a(_: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I) {}
  fn b(_: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I) {}
  #[test]
  fn sanity_check() -> Result<()> {
    let hook = unsafe { GenericDetour::<BigFn>::new(a, b) };
    Ok(())
  }
}
#[cfg(feature = "42-args")]
mod args_42 {
  use super::*;
  use retour::GenericDetour;


  type I = i32;
  type BiggerFn = fn(I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I);

  fn a(_: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I,
    _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I) {}
  fn b(_: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I,
    _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I, _: I) {}
  #[test]
  fn sanity_check() -> Result<()> {
    let hook = unsafe { GenericDetour::<BiggerFn>::new(a, b)? };
    Ok(())
  }
}

#[cfg(target_arch="x86_64")]
mod relative_ip {
  use std::arch::global_asm;
  use super::*;
  use retour::GenericDetour;

  unsafe extern "C" {
    unsafe fn relative_ip_add_3(x: i32, y: i32) -> i32;
  }

  static mut RELATIVE_IP_VAR_C: i32 = 0;

  #[cfg(target_family="windows")]
  global_asm!(r#"
      .global relative_ip_add_3
      relative_ip_add_3:
        mov dword ptr [rip - {var_c}], 3    // C7 05 XX XX XX XX 03 00 00 00
        xor rax, rax                        // 48 31 C0
        mov eax, dword ptr [rip - {var_c}]  // 8B 05 XX XX XX XX
        add rax, rcx                        // 48 01 C8
        add rax, rdx                        // 48 01 D0
        ret                                 // C3
    "#,
    var_c = sym RELATIVE_IP_VAR_C,
  );

  #[cfg(target_family="unix")]
  global_asm!(r#"
      .global relative_ip_add_3
      relative_ip_add_3:
        mov dword ptr [rip - {var_c}], 3    // C7 05 XX XX XX XX 03 00 00 00
        xor rax, rax                        // 48 31 C0
        mov eax, dword ptr [rip - {var_c}]  // 8B 05 XX XX XX XX
        add rax, rdi                        // 48 01 F8
        add rax, rsi                        // 48 01 F0
        ret                                 // C3
    "#,
    var_c = sym RELATIVE_IP_VAR_C,
  );

  type UnsafeFnAdd = unsafe extern "C" fn(i32, i32) -> i32;

  #[test]
  fn test() -> Result<()> {
    unsafe {
      let hook = GenericDetour::<UnsafeFnAdd>::new(relative_ip_add_3, sub_detour)
        .expect("target or source is not usable for detouring");

      assert_eq!(relative_ip_add_3(10, 5), 15+3);
      assert_eq!(hook.call(10, 5), 15+3);
      hook.enable()?;
      {
        assert_eq!(hook.call(10, 5), 15+3);
        assert_eq!(relative_ip_add_3(10, 5), 5);
      }
      hook.disable()?;
      assert_eq!(hook.call(10, 5), 15+3);
      assert_eq!(relative_ip_add_3(10, 5), 15+3);
    }

    Ok(())
  }
}
