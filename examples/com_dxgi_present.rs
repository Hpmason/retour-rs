#![cfg(all(windows, feature = "static-detour"))]
//! A `IDXGISwapChain::Present` detour example.
//!
//! Ensure the crate is compiled as a 'cdylib' library to allow C interop.
use std::error::Error;
use std::mem::size_of;
use std::os::raw::c_void;
use std::ptr::null;

use windows::core::{Interface, HRESULT, HSTRING, PCWSTR};
use windows::Win32::Foundation::{BOOL, HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::System::Console::AllocConsole;
use windows::Win32::System::LibraryLoader::{DisableThreadLibraryCalls, GetModuleHandleW};
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::Win32::UI::WindowsAndMessaging::*;

use retour::static_detour;

static_detour! {
    static PresentHook:  unsafe extern "system" fn(*mut c_void, u32, u32) -> HRESULT;
}

#[allow(non_snake_case)]
fn present(This: *mut c_void, SyncInterval: u32, Flags: u32) -> HRESULT {
  println!("present");
  unsafe { PresentHook.call(This, SyncInterval, Flags) }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(module: HMODULE, call_reason: u32, _reserved: *mut c_void) -> BOOL {
  unsafe {
    DisableThreadLibraryCalls(module);
  }

  if call_reason == DLL_PROCESS_ATTACH {
    unsafe {
      AllocConsole();
    }

    std::thread::spawn(|| unsafe {
      match crate::main() {
        Ok(()) => 0 as u32,
        Err(e) => {
          println!("Error occurred when injecting: {}", e);
          1
        },
      }
    });
  }
  true.into()
}

unsafe extern "system" fn window_proc(
  hwnd: HWND,
  msg: u32,
  w_param: WPARAM,
  l_param: LPARAM,
) -> LRESULT {
  // workaround for https://github.com/microsoft/windows-rs/issues/2556
  DefWindowProcW(hwnd, msg, w_param, l_param)
}

unsafe fn main() -> Result<(), Box<dyn Error>> {
  let vtable = get_d3d11_vtables().as_ref().unwrap();
  println!("Found Present Pointer at {:p}", vtable.Present as *const ());
  PresentHook.initialize(vtable.Present, present)?;

  PresentHook.enable()?;

  println!("Hook activated");
  Ok(())
}

unsafe fn get_render_window() -> (WNDCLASSEXW, HWND) {
  let window_class_name = HSTRING::from("DxHookWindowClass");
  let window_class = WNDCLASSEXW {
    cbSize: size_of::<WNDCLASSEXW>() as u32,
    style: CS_HREDRAW | CS_VREDRAW,
    lpfnWndProc: Some(window_proc),
    cbClsExtra: 0,
    cbWndExtra: 0,
    hInstance: GetModuleHandleW(None).unwrap(),
    hIcon: HICON::default(),
    hCursor: HCURSOR::default(),
    hbrBackground: HBRUSH::default(),
    lpszMenuName: PCWSTR(null()),
    lpszClassName: PCWSTR(window_class_name.as_wide().as_ptr()),
    hIconSm: HICON::default(),
  };

  RegisterClassExW(&window_class);

  let hwnd = CreateWindowExW(
    WINDOW_EX_STYLE::default(),
    window_class.lpszClassName,
    PCWSTR(HSTRING::from("DxHookWindowClass").as_wide().as_ptr()),
    WS_OVERLAPPEDWINDOW,
    0,
    0,
    100,
    100,
    HWND::default(),
    HMENU::default(),
    window_class.hInstance,
    None,
  );

  (window_class, hwnd)
}

unsafe fn get_d3d11_vtables() -> *const IDXGISwapChain_Vtbl {
  let (window_class, hwnd) = get_render_window();
  println!("made new hwnd {:?}", hwnd);
  let swapchain_desc = DXGI_SWAP_CHAIN_DESC {
    BufferDesc: DXGI_MODE_DESC {
      Width: 100,
      Height: 100,
      RefreshRate: DXGI_RATIONAL {
        Numerator: 60,
        Denominator: 1,
      },
      Format: DXGI_FORMAT_R8G8B8A8_UNORM,
      ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
      Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
    },
    SampleDesc: DXGI_SAMPLE_DESC {
      Count: 1,
      Quality: 0,
    },
    BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
    BufferCount: 1,
    OutputWindow: hwnd,
    Windowed: true.into(),
    SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
    Flags: DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH.0 as u32,
  };

  let feature_levels = [D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1];
  let mut out_swapchain = None;
  let mut out_device = None;
  let mut out_context: Option<ID3D11DeviceContext> = None;
  //
  D3D11CreateDeviceAndSwapChain(
    None,
    D3D_DRIVER_TYPE_HARDWARE,
    HMODULE::default(),
    D3D11_CREATE_DEVICE_FLAG::default(),
    Some(&feature_levels),
    D3D11_SDK_VERSION,
    Some(&swapchain_desc),
    Some(&mut out_swapchain),
    Some(&mut out_device),
    None,
    Some(&mut out_context),
  )
  .unwrap();
  println!("d3dhresult {:x?}", 0);

  let swapchain = out_swapchain.unwrap();
  let swapchain_vtbl: &IDXGISwapChain_Vtbl = swapchain.vtable();

  CloseWindow(hwnd);
  UnregisterClassW(window_class.lpszClassName, window_class.hInstance);

  swapchain_vtbl
}
