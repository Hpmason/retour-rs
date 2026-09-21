#![feature(c_variadic)]

#[cfg(not(windows))]
mod implementation {
  use retour::static_detour;
  use std::{
    ffi::{CStr, CString, VaList},
    os::raw::{c_char, c_int},
  };

  extern "C" {
    fn printf(fmt_str: *const c_char, args: ...) -> c_int;
  }

  static_detour! {
    static Printftour: unsafe extern "C" fn(*const c_char, ...) -> c_int;
  }

  fn definitely_printf(fmt_str: *const c_char, mut args: VaList<'_>) -> c_int {
    let fmt_str = unsafe { CStr::from_ptr(fmt_str) };
    let arg = unsafe { CStr::from_ptr(args.next_arg::<*const c_char>()) };
    let arg2 = unsafe { CStr::from_ptr(args.next_arg::<*const c_char>()) };
    eprintln!("fmt_str: {fmt_str:?}, arg: {arg:?}, arg2: {arg2:?}");
    0
  }

  pub fn run() {
    unsafe {
      Printftour.initialize(printf, definitely_printf).unwrap();
    }

    let s = CString::new("%s %s").unwrap();
    let arg = CString::new("hello").unwrap();
    let arg2 = CString::new("world").unwrap();

    unsafe {
      printf(s.as_ptr(), arg.as_ptr(), arg2.as_ptr());
      Printftour.enable().unwrap();
      printf(s.as_ptr(), arg.as_ptr(), arg2.as_ptr());
    }
  }
}

fn main() {
  #[cfg(not(windows))]
  implementation::run();
}
