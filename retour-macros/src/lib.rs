use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Attribute, Ident, Token, TypeFnPtr, Visibility, parse::Parse, parse_macro_input};

#[proc_macro]
// The doc is defined in retour re-export
pub fn static_detour(input: TokenStream) -> TokenStream {
  let detours = parse_macro_input!(input as StaticDetours);

  let mut tokens = TokenStream::new();
  for StaticDetour {
    attrs,
    vis,
    name,
    sig,
  } in detours.0
  {
    let unsafety = sig.unsafety;
    let output = &sig.output;
    let abi = &sig.abi;
    let names = (0..sig.inputs.len())
      .map(|i| format_ident!("arg{i}").into_token_stream())
      .collect::<Vec<_>>();
    let args = sig.inputs.iter().map(|v| &v.ty);

    // Optionally add variadic specific components
    let (vlist_arg, vlist_name) = if sig.variadic.is_some() {
      let name = quote! { variadic };
      (quote! { #name: ... }, name)
    } else {
      (quote! {}, quote! {})
    };

    let generated = quote! {
      #[allow(non_upper_case_globals)]
      #(#attrs)*
      #vis static #name: retour::StaticDetour<#sig> = {
        #[inline(never)]
        #[allow(unused_unsafe)]
        #unsafety #abi fn __ffi_detour(#(#names: #args,)* #vlist_arg) #output {
          #[allow(unused_unsafe)]
          (#name.__detour())(#(#names,)* #vlist_name)
        }

        retour::StaticDetour::__new(__ffi_detour)
      };
    };
    tokens.extend(TokenStream::from(generated));
  }

  tokens
}

struct StaticDetour {
  attrs: Vec<Attribute>,
  vis: Visibility,
  name: Ident,
  sig: TypeFnPtr,
}

impl Parse for StaticDetour {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let attrs = input.call(Attribute::parse_outer)?;
    let vis = input.parse()?;
    input.parse::<Token![static]>()?;
    let name = input.parse()?;
    input.parse::<Token![:]>()?;
    let sig = input.parse()?;
    input.parse::<Token![;]>()?;

    Ok(Self {
      attrs,
      vis,
      name,
      sig,
    })
  }
}

struct StaticDetours(Vec<StaticDetour>);

impl Parse for StaticDetours {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let mut detours = Vec::new();
    while !input.is_empty() {
      detours.push(input.parse()?);
    }

    Ok(StaticDetours(detours))
  }
}
