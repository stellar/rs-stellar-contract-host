use itertools::MultiUnzip;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{FnArg, Type};

pub fn derive_contract_function_set<'a>(
    ty: &Box<Type>,
    methods: impl Iterator<Item = &'a syn::ImplItemMethod>,
) -> TokenStream2 {
    let (discriminant_consts, func_calls): (Vec<_>, Vec<_>) = methods
        .map(|m| {
            let ident = &m.sig.ident;
            let name = ident.to_string();
            let discriminant_ident = format_ident!("DISCRIMINANT_{}", name.to_uppercase());
            let discriminant_const = quote! {
                const #discriminant_ident: u64 = soroban_env_common::Symbol::from_str(#name).to_raw().get_payload()
            };
            let (arg_indices, args, arg_types): (Vec<_>, Vec<_>, Vec<_>) = m.sig.inputs.iter().skip(1).cloned().enumerate().map(|(i, a)| {
                let arg = format_ident!("arg{}", i);
                match a {
                    FnArg::Typed(t) => (i, arg, t.ty),
                    _ => panic!(),
                }
            }).multiunzip();
            let num_args = args.len();
            let func_call = quote! {
                #discriminant_ident => {
                    if args.len() == #num_args {
                        #(let #args: #arg_types = args.get(#arg_indices).cloned().ok_or(())?.in_env(host).try_into().map_err(|_| ())?;)*
                        Self::#ident(host, #(#args,)*)?.try_into_val(host).map_err(|_| ())
                    } else {
                        Err(())
                    }
                }
            };
            (discriminant_const, func_call)
        })
        .multiunzip();
    quote! {
        impl crate::native_contract::NativeContract for #ty {
            fn call(
                &self,
                func: &soroban_env_common::Symbol,
                host: &crate::Host,
                args: &[soroban_env_common::RawVal],
            ) -> Result<soroban_env_common::RawVal, ()> {
                use super::*;
                #(#discriminant_consts;)*
                match func.to_raw().get_payload() {
                    #(#func_calls)*
                    _ => panic!(),
                }
            }
        }
    }
}
