macro_rules! impl_hookable {
  (@recurse () ($($nm:ident : $ty:ident),*)) => {
    impl_hookable!(@impl_all ($($nm : $ty),*));
  };
  (@recurse
      ($hd_nm:ident : $hd_ty:ident $(, $tl_nm:ident : $tl_ty:ident)*)
      ($($nm:ident : $ty:ident),*)) => {
    impl_hookable!(@impl_all ($($nm : $ty),*));
    impl_hookable!(@recurse ($($tl_nm : $tl_ty),*) ($($nm : $ty,)* $hd_nm : $hd_ty));
  };

  (@impl_all ($($nm:ident : $ty:ident),*)) => {
    impl_hookable!(@impl_pair ($($nm : $ty),*) (extern "C"        fn($($ty),*) -> Ret));
    impl_hookable!(@impl_pair ($($nm : $ty),*) (extern "Rust"     fn($($ty),*) -> Ret));
    impl_hookable!(@impl_pair ($($nm : $ty),*) (extern "system"   fn($($ty),*) -> Ret));
    #[cfg(target_arch = "x86")]
    impl_hookable!(@impl_pair ($($nm : $ty),*) (extern "cdecl"    fn($($ty),*) -> Ret));
    #[cfg(target_arch = "x86")]
    impl_hookable!(@impl_pair ($($nm : $ty),*) (extern "fastcall" fn($($ty),*) -> Ret));
    #[cfg(target_arch = "x86")]
    impl_hookable!(@impl_pair ($($nm : $ty),*) (extern "stdcall"  fn($($ty),*) -> Ret));
    #[cfg(target_arch = "x86_64")]
    impl_hookable!(@impl_pair ($($nm : $ty),*) (extern "win64"    fn($($ty),*) -> Ret));

    #[cfg(any(docsrs, all(target_arch = "x86", feature = "thiscall-abi")))]
    impl_hookable!(@impl_pair ($($nm : $ty),*) (extern "thiscall" fn($($ty),*) -> Ret));
  };

  (@impl_pair ($($nm:ident : $ty:ident),*) ($($fn_t:tt)*)) => {
    impl_hookable!(@impl_fun ($($nm : $ty),*) ($($fn_t)*) (unsafe $($fn_t)*));
  };

  (@impl_fun ($($nm:ident : $ty:ident),*) ($safe_type:ty) ($unsafe_type:ty)) => {
    impl_hookable!(@impl_core ($($nm : $ty),*) ($safe_type));
    impl_hookable!(@impl_core ($($nm : $ty),*) ($unsafe_type));

    impl_hookable!(@impl_unsafe ($($nm : $ty),*) ($unsafe_type) ($safe_type));
    impl_hookable!(@impl_safe ($($nm : $ty),*) ($safe_type));
  };

  (@impl_unsafe ($($nm:ident : $ty:ident),*) ($target:ty) ($detour:ty)) => {
    #[cfg(feature = "static-detour")]
    impl<Ret: 'static, $($ty: 'static),*> $crate::StaticDetour<$target> {
      #[doc(hidden)]
      pub unsafe fn call(&self, $($nm : $ty),*) -> Ret {
        let original: $target = ::std::mem::transmute(self.trampoline().expect("calling detour trampoline"));
        original($($nm),*)
      }
    }

    impl<Ret: 'static, $($ty: 'static),*> $crate::GenericDetour<$target> {
      #[doc(hidden)]
      pub unsafe fn call(&self, $($nm : $ty),*) -> Ret {
        let original: $target = ::std::mem::transmute(self.trampoline());
        original($($nm),*)
      }
    }
  };

  (@impl_safe ($($nm:ident : $ty:ident),*) ($fn_type:ty)) => {
    #[cfg(feature = "static-detour")]
    impl<Ret: 'static, $($ty: 'static),*> $crate::StaticDetour<$fn_type> {
      #[doc(hidden)]
      pub fn call(&self, $($nm : $ty),*) -> Ret {
        unsafe {
          let original: $fn_type = ::std::mem::transmute(self.trampoline().expect("calling detour trampoline"));
          original($($nm),*)
        }
      }
    }

    impl<Ret: 'static, $($ty: 'static),*> $crate::GenericDetour<$fn_type> {
      #[doc(hidden)]
      pub fn call(&self, $($nm : $ty),*) -> Ret {
        unsafe {
          let original: $fn_type = ::std::mem::transmute(self.trampoline());
          original($($nm),*)
        }
      }
    }
  };

  (@impl_core ($($nm:ident : $ty:ident),*) ($fn_type:ty)) => {
    unsafe impl<Ret: 'static, $($ty: 'static),*> Function for $fn_type {
      type Arguments = ($($ty,)*);
      type Output = Ret;

      unsafe fn from_ptr(ptr: *const ()) -> Self {
        ::std::mem::transmute(ptr)
      }

      fn to_ptr(&self) -> *const () {
        *self as *const ()
      }
    }
  };

  ($($nm:ident : $ty:ident),*) => {
    impl_hookable!(@recurse ($($nm : $ty),*) ());
  };
}
