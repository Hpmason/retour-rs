#![cfg(windows)]
#![allow(non_upper_case_globals)]

use once_cell::sync::Lazy;
use retour::{GenericDetour, Function};
use winapi::{shared::{ntdef::*, minwindef::*}};
use minidl::Library;

type fn_LoadLibraryA = extern "system" fn(LPCSTR) -> HMODULE;
type fn_LoadLibraryW = extern "system" fn(LPCWSTR) -> HMODULE;
type fn_LoadLibraryExA = extern "system" fn(LPCSTR, HANDLE, DWORD) -> HMODULE;
type fn_LoadLibraryExW = extern "system" fn(LPCWSTR, HANDLE, DWORD) -> HMODULE;

static hook_LoadLibraryA: Lazy<GenericDetour<fn_LoadLibraryA>> = Lazy::new(|| build_detour("kernel32.dll\0", "LoadLibraryA\0", our_LoadLibraryA));
static hook_LoadLibraryW: Lazy<GenericDetour<fn_LoadLibraryW>> = Lazy::new(|| build_detour("kernel32.dll\0", "LoadLibraryW\0", our_LoadLibraryW));
static hook_LoadLibraryExA: Lazy<GenericDetour<fn_LoadLibraryExA>> = Lazy::new(|| build_detour("kernel32.dll\0", "LoadLibraryExA\0", our_LoadLibraryExA));
static hook_LoadLibraryExW: Lazy<GenericDetour<fn_LoadLibraryExW>> = Lazy::new(|| build_detour("kernel32.dll\0", "LoadLibraryExW\0", our_LoadLibraryExW));

fn build_detour<T: Function>(lpFileName: &str, lpProcName: &str, detour_fn: T) -> GenericDetour<T> {
  let library = Library::load(lpFileName).unwrap();
  let ori = unsafe { library.sym(lpProcName).unwrap() };
  return unsafe { 
    GenericDetour::new(ori, detour_fn).unwrap()
  };
}

fn strlen(s: *const i8) -> usize {
  let mut i = 0;
  unsafe {
      while *s.offset(i) != 0 {
          i += 1;
      }
  }
  i as usize
}

fn wcslen(s: *const u16) -> usize {
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
  let length = unsafe { strlen(input) };
  let slice: &[u8] = unsafe { std::slice::from_raw_parts(input as *const u8, length) };
  return String::from_utf8(slice.to_vec()).unwrap();
}

fn lpcwstr_to_rust_string(input: LPCWSTR) -> String {
  if input.is_null() {
    return String::from("(null)");
  }
  let length = unsafe { wcslen(input) };
  let slice = unsafe { std::slice::from_raw_parts(input, length) };
  return String::from_utf16_lossy(slice);
}

extern "system" fn our_LoadLibraryA(lpFileName: LPCSTR) -> HMODULE {
  println!("our_LoadLibraryA lpFileName = {}", lpcstr_to_rust_string(lpFileName));
  unsafe { hook_LoadLibraryA.disable().unwrap() };
  let ret_val = hook_LoadLibraryA.call(lpFileName);
  println!("our_LoadLibraryA lpFileName = {} ret_val = {:p}", lpcstr_to_rust_string(lpFileName), ret_val);
  unsafe { hook_LoadLibraryA.enable().unwrap() };
  return ret_val;
}

extern "system" fn our_LoadLibraryW(lpFileName: LPCWSTR) -> HMODULE {
  println!("our_LoadLibraryW lpFileName = {}", lpcwstr_to_rust_string(lpFileName));
  unsafe { hook_LoadLibraryW.disable().unwrap() };
  let ret_val = hook_LoadLibraryW.call(lpFileName);
  println!("our_LoadLibraryW lpFileName = {} ret_val = {:p}", lpcwstr_to_rust_string(lpFileName), ret_val);
  unsafe { hook_LoadLibraryW.enable().unwrap() };
  return ret_val;
}

extern "system" fn our_LoadLibraryExA(lpLibFileName: LPCSTR, hFile: HANDLE, dwFlags: DWORD) -> HMODULE {
  println!(
    "our_LoadLibraryExA lpLibFileName = {} hFile = {:p} dwFlags = {:08x}",
    lpcstr_to_rust_string(lpLibFileName),
    hFile,
    dwFlags
  );
  unsafe { hook_LoadLibraryExA.disable().unwrap(); }
  let ret_val = hook_LoadLibraryExA.call(lpLibFileName, hFile, dwFlags);
  println!(
    "our_LoadLibraryExA lpLibFileName = {} hFile = {:p} dwFlags = {:08x} ret_val = {:p}",
    lpcstr_to_rust_string(lpLibFileName),
    hFile,
    dwFlags,
    ret_val
  );
  unsafe { hook_LoadLibraryExA.enable().unwrap(); }
  return ret_val;
}

extern "system" fn our_LoadLibraryExW(lpLibFileName: LPCWSTR, hFile: HANDLE, dwFlags: DWORD) -> HMODULE {
  println!(
    "our_LoadLibraryExW lpLibFileName = {} hFile = {:p} dwFlags = {:08x}",
    lpcwstr_to_rust_string(lpLibFileName),
    hFile,
    dwFlags
  );
  unsafe { hook_LoadLibraryExW.disable().unwrap(); }
  let ret_val = hook_LoadLibraryExW.call(lpLibFileName, hFile, dwFlags);
  println!(
    "our_LoadLibraryExW lpLibFileName = {} hFile = {:p} dwFlags = {:08x} ret_val = {:p}",
    lpcwstr_to_rust_string(lpLibFileName),
    hFile,
    dwFlags,
    ret_val
  );
  unsafe { hook_LoadLibraryExW.enable().unwrap(); }
  return ret_val;
}

pub fn init() {
  unsafe { hook_LoadLibraryA.enable().unwrap(); }
  unsafe { hook_LoadLibraryW.enable().unwrap(); }
  unsafe { hook_LoadLibraryExA.enable().unwrap(); }
  unsafe { hook_LoadLibraryExW.enable().unwrap(); }
}
