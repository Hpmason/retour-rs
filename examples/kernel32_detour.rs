#![cfg(windows)]
#![allow(non_upper_case_globals, non_snake_case, non_camel_case_types)]

use once_cell::sync::Lazy;
use retour::GenericDetour;
use winapi::um::libloaderapi::{GetProcAddress, LoadLibraryA};
use winapi::shared::minwindef::DWORD;
use winapi::shared::minwindef::HMODULE;
use winapi::shared::minwindef::HINSTANCE;
use winapi::shared::minwindef::LPVOID;
use winapi::shared::minwindef::BOOL;
use winapi::shared::ntdef::TRUE;
use winapi::shared::ntdef::LPCSTR;
use winapi::um::winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH};

type fn_LoadLibraryA = extern "system" fn(LPCSTR) -> HMODULE;

static hook_LoadLibraryA: Lazy<GenericDetour<fn_LoadLibraryA>> = Lazy::new(|| {
  let library_handle = unsafe { LoadLibraryA("kernel32.dll\0".as_ptr() as _) };
  let address = unsafe { GetProcAddress(library_handle, "LoadLibraryA\0".as_ptr() as _) };
  let ori: fn_LoadLibraryA = unsafe { std::mem::transmute(address) };
  return unsafe { 
    GenericDetour::new(ori, our_LoadLibraryA).unwrap()
  };
});

fn strlen(s: *const i8) -> usize {
  let mut i = 0;
  unsafe {
      while *s.offset(i) != 0 {
          i += 1;
      }
  }
  i as usize
}

fn lpcstr_to_rust_string(input: LPCSTR) -> String {
  if input.is_null() {
    return String::from("(null)");
  }
  let length = strlen(input);
  let slice: &[u8] = unsafe { std::slice::from_raw_parts(input as *const u8, length) };
  return String::from_utf8(slice.to_vec()).unwrap();
}

extern "system" fn our_LoadLibraryA(lpFileName: LPCSTR) -> HMODULE {
  println!("our_LoadLibraryA lpFileName = {}", lpcstr_to_rust_string(lpFileName));
  unsafe { hook_LoadLibraryA.disable().unwrap() };
  let ret_val = hook_LoadLibraryA.call(lpFileName);
  println!("our_LoadLibraryA lpFileName = {} ret_val = {:p}", lpcstr_to_rust_string(lpFileName), ret_val);
  unsafe { hook_LoadLibraryA.enable().unwrap() };
  return ret_val;
}

#[no_mangle]
unsafe extern "system" fn DllMain(_hinst: HINSTANCE, reason: DWORD, _reserved: LPVOID) -> BOOL {
  match reason {
    DLL_PROCESS_ATTACH => {
      println!("attaching");
      unsafe { hook_LoadLibraryA.enable().unwrap(); }
    }
    DLL_PROCESS_DETACH => {
      println!("detaching");
    }
    DLL_THREAD_ATTACH => {}
    DLL_THREAD_DETACH => {}
    _ => {}
  };
  return TRUE as BOOL;
}
