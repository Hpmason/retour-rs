//! Traits describing detours and applicable functions.
//!
//! Several of the traits in this module are automatically implemented and
//! should generally not be implemented by users of this library.

/// Trait representing a function that can be used as a target or detour for
/// detouring.
pub unsafe trait Function: Sized + Copy + Sync + 'static {
  /// The argument types as a tuple.
  type Arguments;

  /// The return type.
  type Output;

  /// Constructs a `Function` from an untyped pointer.
  unsafe fn from_ptr(ptr: *const ()) -> Self;

  /// Returns an untyped pointer for this function.
  fn to_ptr(&self) -> *const ();
}

/// Trait indicating that `Self` can be detoured by the given function `D`.
pub unsafe trait HookableWith<D: Function>: Function {}

unsafe impl<T: Function> HookableWith<T> for T {}

include!(concat!(env!("OUT_DIR"), "/impl_hookable.rs"));
