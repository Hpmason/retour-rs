use std::{env, fs, path::Path};

use quote::{format_ident, quote, ToTokens};

fn main() {
  impl_hookable();
}

const MAX_ARG_COUNT: usize = 42;

fn impl_hookable() {
  let abis = [
    "C", "Rust", "system", "cdecl", "fastcall", "stdcall", "win64", "thiscall",
  ];

  let (args, names) = (0..MAX_ARG_COUNT)
    .map(|i| {
      (
        format_ident!("Arg{i}").into_token_stream(),
        format_ident!("arg{i}").into_token_stream(),
      )
    })
    .collect::<(Vec<_>, Vec<_>)>();

  let mut tokens = Vec::new();

  // this will iterate like: <empty>, A, AB, ABC, ABCD, ...
  for i in 0..=MAX_ARG_COUNT {
    let args = &args[..i];
    let names = &names[..i];

    let impl_generic = quote! { impl<Ret: 'static, #(#args: 'static),*> };

    // Features are built in source to avoid re-running build.rs on feature change
    let feats = match args.len() {
      ..=14 => quote! {},
      15..=28 => quote! { #[cfg(feature = "28-args")] },
      29..=42 => quote! { #[cfg(feature = "42-args")] },
      _ => unreachable!(),
    };

    for abi in abis.iter().copied() {
      let feats = match abi {
        "cdecl" | "fastcall" | "stdcall" => quote! {
          #feats
          #[cfg(target_arch = "x86")]
        },
        "win64" => quote! {
          #feats
          #[cfg(target_arch = "x86_64")]
        },
        "thiscall" => quote! {
          #feats
          #[cfg(any(docsrs, all(target_arch = "x86", feature = "thiscall-abi")))]
        },
        _ => feats.clone(),
      };

      for safe in [false, true] {
        let safety = if safe {
          quote! {}
        } else {
          quote! { unsafe }
        };

        for variadic in [false, true] {
          if variadic && (abi != "C" || safe) {
            continue;
          }

          // Optionally add variadic specific components
          let (vlist_call_sig, vlist_abi, vlist_arg, vlist_name, vlist_syntax, feats) = if variadic
          {
            let vlist_syntax = quote! { ... };
            let vlist_name = quote! { variadic };
            (
              quote! { #vlist_name: #vlist_syntax },
              quote! { extern "C" },
              quote! { std::ffi::VaList<'a>, },
              vlist_name,
              vlist_syntax,
              quote! {
                  #feats
                  #[cfg(feature = "c-variadic")]
              },
            )
          } else {
            (
              quote! {},
              quote! {},
              quote! {},
              quote! {},
              quote! {},
              feats.clone(),
            )
          };

          let fn_type = quote! { #safety extern #abi fn(#(#args,)* #vlist_syntax) -> Ret };
          let call_sig = quote! {
              #[doc(hidden)]
              pub #safety #vlist_abi fn call(&self, #(#names : #args,)* #vlist_call_sig) -> Ret
          };
          let call_original = quote! { original(#(#names,)* #vlist_name) };

          // Final result of this iteration
          let res = quote! {
            #[cfg(feature = "static-detour")]
            #feats
            #impl_generic StaticDetour<#fn_type> {
              #call_sig {
                unsafe {
                  let original: #fn_type = ::std::mem::transmute(self.trampoline().expect("calling detour trampoline"));
                  #call_original
                }
              }
            }

            #feats
            #impl_generic GenericDetour<#fn_type> {
              #call_sig {
                unsafe {
                  let original: #fn_type = ::std::mem::transmute(self.trampoline());
                  #call_original
                }
              }
            }

            #feats
            unsafe #impl_generic Function for #fn_type {
              type Arguments<'a> = (#(#args,)* #vlist_arg);
              type Output = Ret;

              unsafe fn from_ptr(ptr: *const ()) -> Self {
                ::std::mem::transmute(ptr)
              }

              fn to_ptr(&self) -> *const () {
                *self as *const ()
              }
            }
          };

          tokens.push(res);
        }
      }
    }
  }

  // Collect generated implementations into a module
  let tokens = quote! {
    mod impl_hookable {
      // We let clippy run on generated code as this would allow for catching clippy warnings and improve generated code
      //
      // Some are allowed for lints reasonable to be allowed for generated code
      #![allow(
        clippy::type_complexity,
        clippy::too_many_arguments
      )]

      use crate::*;

      #(#tokens)*
    }
  };

  let out = Path::new(&env::var_os("OUT_DIR").unwrap()).join("impl_hookable.rs");

  let tokens_raw = tokens.to_string();

  // Giving a clean source is worth for the sake of having a sane error output
  let source = match syn::parse2(tokens) {
    Ok(source) => prettyplease::unparse(&source),
    Err(_) => {
      // Could not parse as there's a syntax error...
      // Give raw string to show syntax error during compilation
      tokens_raw
    },
  };

  fs::write(out, source).unwrap();
}
