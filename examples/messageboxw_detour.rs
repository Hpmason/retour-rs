#![cfg(all(windows, feature = "nightly"))]
//! A `MessageBoxW` detour example.
//!
//! Ensure the crate is compiled as a 'cdylib' library to allow C interop.
use retour::static_detour;
use std::error::Error;
use std::ffi::c_int;
use std::{ffi::CString, iter, mem};
use windows::core::{PCSTR, PCWSTR};
use windows::w;
use windows::Win32::Foundation::{BOOL, HANDLE, HWND};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::SystemServices::{
  DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH,
};

static_detour! {
  static MessageBoxWHook: unsafe extern "system" fn(HWND, PCWSTR, PCWSTR, u32) -> c_int;
}

// A type alias for `MessageBoxW` (makes the transmute easy on the eyes)
type FnMessageBoxW = unsafe extern "system" fn(HWND, PCWSTR, PCWSTR, u32) -> c_int;

/// Called when the DLL is attached to the process.
unsafe fn main() -> Result<(), Box<dyn Error>> {
  // Retrieve an absolute address of `MessageBoxW`. This is required for
  // libraries due to the import address table. If `MessageBoxW` would be
  // provided directly as the target, it would only hook this DLL's
  // `MessageBoxW`. Using the method below an absolute address is retrieved
  // instead, detouring all invocations of `MessageBoxW` in the active process.
  let address = get_module_symbol_address("user32.dll", "MessageBoxW")
    .expect("could not find 'MessageBoxW' address");
  let target: FnMessageBoxW = mem::transmute(address);

  // Initialize AND enable the detour (the 2nd parameter can also be a closure)
  MessageBoxWHook
    .initialize(target, messageboxw_detour)?
    .enable()?;
  Ok(())
}

/// Called whenever `MessageBoxW` is invoked in the process.
fn messageboxw_detour(hwnd: HWND, text: PCWSTR, _caption: PCWSTR, msgbox_style: u32) -> c_int {
  // Call the original `MessageBoxW`, but replace the caption
  let replaced_caption = w!("Detoured!");
  unsafe { MessageBoxWHook.call(hwnd, text, replaced_caption, msgbox_style) }
}

/// Returns a module symbol's absolute address.
fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
  let module = module
    .encode_utf16()
    .chain(iter::once(0))
    .collect::<Vec<u16>>();
  let symbol = CString::new(symbol).unwrap();
  unsafe {
    let handle = GetModuleHandleW(PCWSTR(module.as_ptr() as _)).unwrap();
    match GetProcAddress(handle, PCSTR(symbol.as_ptr() as _)) {
      Some(func) => Some(func as usize),
      None => None,
    }
  }
}

#[no_mangle]
unsafe extern "system" fn DllMain(_hinst: HANDLE, reason: u32, _reserved: u64) -> BOOL {
  match reason {
    DLL_PROCESS_ATTACH => {
      println!("attaching");
      unsafe { main().unwrap() }
    },
    DLL_PROCESS_DETACH => {
      println!("detaching");
    },
    DLL_THREAD_ATTACH => {},
    DLL_THREAD_DETACH => {},
    _ => {},
  };
  return BOOL::from(true);
}
