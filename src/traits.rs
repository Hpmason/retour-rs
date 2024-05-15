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

#[cfg(not(feature = "28-args"))]
impl_hookable! {
  __arg_0:  A, __arg_1:  B, __arg_2:  C, __arg_3:  D, __arg_4:  E, __arg_5:  F, __arg_6:  G,
  __arg_7:  H, __arg_8:  I, __arg_9:  J, __arg_10: K, __arg_11: L, __arg_12: M, __arg_13: N
}

#[cfg(all(feature = "28-args", not(feature = "42-args")))]
impl_hookable! {
  __arg_0:  A, __arg_1:  B, __arg_2:  C, __arg_3:  D, __arg_4:  E, __arg_5:  F, __arg_6:  G,
  __arg_7:  H, __arg_8:  I, __arg_9:  J, __arg_10: K, __arg_11: L, __arg_12: M, __arg_13: N,
  __arg_14: O, __arg_15: P, __arg_16: Q, __arg_17: R, __arg_18: S, __arg_19: T, __arg_20: U,
  __arg_21: V, __arg_22: W, __arg_23: X, __arg_24: Y, __arg_25: Z, __arg_26: AA, __arg27: AB
}

#[cfg(feature = "42-args")]
impl_hookable! {
  __arg_0:   A, __arg_1:   B, __arg_2:   C, __arg_3:   D, __arg_4:   E, __arg_5:   F, __arg_6:   G,
  __arg_7:   H, __arg_8:   I, __arg_9:   J, __arg_10:  K, __arg_11:  L, __arg_12:  M, __arg_13:  N,
  __arg_14:  O, __arg_15:  P, __arg_16:  Q, __arg_17:  R, __arg_18:  S, __arg_19:  T, __arg_20:  U,
  __arg_21:  V, __arg_22:  W, __arg_23:  X, __arg_24:  Y, __arg_25:  Z, __arg_26: AA, __arg_27: AB,
  __arg_28: AC, __arg_29: AD, __arg_30: AE, __arg_31: AF, __arg_32: AG, __arg_33: AH, __arg_34: AI,
  __arg_35: AJ, __arg_36: AK, __arg_37: AL, __arg_38: AM, __arg_39: AN, __arg_40: AO, __arg_41: AP
}
