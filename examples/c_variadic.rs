#![cfg(feature = "c-variadic")]
#![feature(c_variadic)]
use std::ffi::CStr;
use std::os::raw::c_char;
use std::os::raw::c_int;

use retour::GenericDetour;

extern "C" {
    fn printf(fmt_str: *const c_char, args:...) -> c_int;
}
// static_detour! {
//     static Opentour: unsafe extern "C" fn(*const c_char ...) -> c_int;
// }

unsafe extern "C" fn definitely_printf(fmt_str: *const c_char, mut args: ...) -> c_int {
    let fmt_string = CStr::from_ptr(fmt_str);
    eprintln!("Format string: {fmt_string:?}");
    args.as_va_list();
    0
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let dt = 
            GenericDetour::<unsafe extern "C" fn(*const c_char, ...) -> c_int>::new(
                printf, 
                definitely_printf
            )?;
        dt.enable()?;
        printf(b"%s\0".as_ptr() as *const c_char, "Hello there!");
    }
    Ok(())
}
