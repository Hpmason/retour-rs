#![allow(non_upper_case_globals)]

use crate::libc::*;
use once_cell::sync::Lazy;
use retour::GenericDetour;
use winapi::{shared::{ntdef::*, minwindef::*}, um::libloaderapi::{LoadLibraryA, GetProcAddress}};

// LoadLibraryA
type fn_LoadLibraryA = extern "system" fn(LPCSTR) -> HMODULE;
static hook_LoadLibraryA: Lazy<GenericDetour<fn_LoadLibraryA>> = Lazy::new(|| {
  let library_handle = unsafe { LoadLibraryA("kernel32.dll\0".as_ptr() as _) };
  let address = unsafe { GetProcAddress(library_handle, "LoadLibraryA\0".as_ptr() as _) };
  let ori: fn_LoadLibraryA = unsafe { std::mem::transmute(address) };
  return unsafe { 
    GenericDetour::new(ori, our_LoadLibraryA).unwrap()
  };
});
extern "system" fn our_LoadLibraryA(lpFileName: LPCSTR) -> HMODULE {
  log::info!("our_LoadLibraryA lpFileName = {}", lpcstr_to_rust_string(lpFileName));
  unsafe { hook_LoadLibraryA.disable().unwrap() };
  let ret_val = hook_LoadLibraryA.call(lpFileName);
  log::info!("our_LoadLibraryA lpFileName = {} ret_val = {:p}", lpcstr_to_rust_string(lpFileName), ret_val);
  unsafe { hook_LoadLibraryA.enable().unwrap() };
  return ret_val;
}

// LoadLibraryW
type fn_LoadLibraryW = extern "system" fn(LPCWSTR) -> HMODULE;
static hook_LoadLibraryW: Lazy<GenericDetour<fn_LoadLibraryW>> = Lazy::new(|| {
  let library_handle = unsafe { LoadLibraryA("kernel32.dll\0".as_ptr() as _) };
  let address = unsafe { GetProcAddress(library_handle, "LoadLibraryW\0".as_ptr() as _) };
  let ori: fn_LoadLibraryW = unsafe { std::mem::transmute(address) };
  return unsafe { 
    GenericDetour::new(ori, our_LoadLibraryW).unwrap()
  };
});
extern "system" fn our_LoadLibraryW(lpFileName: LPCWSTR) -> HMODULE {
  log::info!("our_LoadLibraryW lpFileName = {}", lpcwstr_to_rust_string(lpFileName));
  unsafe { hook_LoadLibraryW.disable().unwrap() };
  let ret_val = hook_LoadLibraryW.call(lpFileName);
  log::info!("our_LoadLibraryW lpFileName = {} ret_val = {:p}", lpcwstr_to_rust_string(lpFileName), ret_val);
  unsafe { hook_LoadLibraryW.enable().unwrap() };
  return ret_val;
}

// LoadLibraryExA
type fn_LoadLibraryExA = extern "system" fn(LPCSTR, HANDLE, DWORD) -> HMODULE;
static hook_LoadLibraryExA: Lazy<GenericDetour<fn_LoadLibraryExA>> = Lazy::new(|| {
  let library_handle = unsafe { LoadLibraryA("kernel32.dll\0".as_ptr() as _) };
  let address = unsafe { GetProcAddress(library_handle, "LoadLibraryExA\0".as_ptr() as _) };
  let ori: fn_LoadLibraryExA = unsafe { std::mem::transmute(address) };
  return unsafe { 
    GenericDetour::new(ori, our_LoadLibraryExA).unwrap()
  };
});
extern "system" fn our_LoadLibraryExA(lpLibFileName: LPCSTR, hFile: HANDLE, dwFlags: DWORD) -> HMODULE {
  log::info!(
    "our_LoadLibraryExA lpLibFileName = {} hFile = {:p} dwFlags = {:08x}",
    lpcstr_to_rust_string(lpLibFileName),
    hFile,
    dwFlags
  );
  unsafe { hook_LoadLibraryExA.disable().unwrap(); }
  let ret_val = hook_LoadLibraryExA.call(lpLibFileName, hFile, dwFlags);
  log::info!(
    "our_LoadLibraryExA lpLibFileName = {} hFile = {:p} dwFlags = {:08x} ret_val = {:p}",
    lpcstr_to_rust_string(lpLibFileName),
    hFile,
    dwFlags,
    ret_val
  );
  unsafe { hook_LoadLibraryExA.enable().unwrap(); }
  return ret_val;
}

// LoadLibraryExW
type fn_LoadLibraryExW = extern "system" fn(LPCWSTR, HANDLE, DWORD) -> HMODULE;
static hook_LoadLibraryExW: Lazy<GenericDetour<fn_LoadLibraryExW>> = Lazy::new(|| {
  let library_handle = unsafe { LoadLibraryA("kernel32.dll\0".as_ptr() as _) };
  let address = unsafe { GetProcAddress(library_handle, "LoadLibraryExW\0".as_ptr() as _) };
  let ori: fn_LoadLibraryExW = unsafe { std::mem::transmute(address) };
  return unsafe { 
    GenericDetour::new(ori, our_LoadLibraryExW).unwrap()
  };
});
extern "system" fn our_LoadLibraryExW(lpLibFileName: LPCWSTR, hFile: HANDLE, dwFlags: DWORD) -> HMODULE {
  log::info!(
    "our_LoadLibraryExW lpLibFileName = {} hFile = {:p} dwFlags = {:08x}",
    lpcwstr_to_rust_string(lpLibFileName),
    hFile,
    dwFlags
  );
  unsafe { hook_LoadLibraryExW.disable().unwrap(); }
  let ret_val = hook_LoadLibraryExW.call(lpLibFileName, hFile, dwFlags);
  log::info!(
    "our_LoadLibraryExW lpLibFileName = {} hFile = {:p} dwFlags = {:08x} ret_val = {:p}",
    lpcwstr_to_rust_string(lpLibFileName),
    hFile,
    dwFlags,
    ret_val
  );
  unsafe { hook_LoadLibraryExW.enable().unwrap(); }
  return ret_val;
}

fn lpcstr_to_rust_string(input: LPCSTR) -> String {
  if input.is_null() {
    return String::from("(null)");
  }
  let length = strlen(input);
  let slice: &[u8] = unsafe { std::slice::from_raw_parts(input as *const u8, length) };
  return String::from_utf8(slice.to_vec()).unwrap();
}

fn lpcwstr_to_rust_string(input: LPCWSTR) -> String {
  if input.is_null() {
    return String::from("(null)");
  }
  let length = wcslen(input);
  let slice = unsafe { std::slice::from_raw_parts(input, length) };
  return String::from_utf16_lossy(slice);
}

pub fn init() {
  Lazy::force(&hook_LoadLibraryA);
  Lazy::force(&hook_LoadLibraryW);
  Lazy::force(&hook_LoadLibraryExA);
  Lazy::force(&hook_LoadLibraryExW);
  unsafe { hook_LoadLibraryA.enable().unwrap(); }
  unsafe { hook_LoadLibraryW.enable().unwrap(); }
  unsafe { hook_LoadLibraryExA.enable().unwrap(); }
  unsafe { hook_LoadLibraryExW.enable().unwrap(); }
}
