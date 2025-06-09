#[macro_use]
extern crate quote;

#[macro_use]
extern crate syn;
extern crate proc_macro2;

extern crate proc_macro;

use proc_macro2::TokenStream;
use syn::{Data, DeriveInput, Expr, Ident};


/// Derive a new struct containing all `Option` fields
/// Mark any field with `#[config_default(expr)]` to set a default configuration value
/// Any field that does not implement `Default` must contain such a attribute 
#[proc_macro_derive(OptionalConfig, attributes(config_default))]
pub fn optional_config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let optional_fields = optional_fields(&input.data);
    let unwrap_code = unwrap_fields(&input.data, format_ident!("val")); let new_name = format_ident!("{}Option", name);
    let default_fields = default_impl(&input.data);

    quote! {
        #[derive(Clone, Serialize, Deserialize)]
        pub struct  #new_name {
            #optional_fields
        }

        impl Default for #name {
            fn default() -> Self {
                Self {
                    #default_fields
                }
            }
        }

        impl From<#new_name> for #name {
            fn from(val: #new_name) -> Self {
                Self {
                    #unwrap_code
                }
            }

        }
    }
    .into()
}

fn optional_fields(data: &Data) -> TokenStream {
    match data {
        Data::Struct(struct_data) => match &struct_data.fields {
            syn::Fields::Named(fields_named) => {
                let recurse = fields_named.named.iter().map(|f| {
                    let name = &f.ident;
                    let ty = &f.ty;
                    quote!( #name: Option<#ty>)
                });

                quote! {
                    #(#recurse,)*
                }
            }
            syn::Fields::Unnamed(_) => unimplemented!(),
            syn::Fields::Unit => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}

fn unwrap_fields(data: &Data, val_ident: Ident) -> TokenStream {
    match data {
        Data::Struct(struct_data) => match &struct_data.fields {
            syn::Fields::Named(fields_named) => {
                let recurse = fields_named.named.iter().map(|f| {
                    let name = &f.ident;
                    let function_call = f
                        .attrs
                        .iter()
                        .find(|&attr| {
                            attr.meta.require_list().is_ok_and(|named| {
                                named.path.get_ident().is_some_and(|ident| {
                                    ident.to_string().as_str() == "config_default"
                                })
                            })
                        })
                        .map_or(quote!(unwrap_or_default()),|attr| {
                            let inner = attr.meta.require_list().unwrap();
                            let default: Expr = inner.parse_args().unwrap();
                            quote! {unwrap_or(#default)}
                        });
                    quote!( #name: #val_ident.#name.#function_call)
                });

                quote! {
                    #(#recurse,)*
                }
            }
            syn::Fields::Unnamed(_) => unimplemented!(),
            syn::Fields::Unit => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}

fn default_impl(data: &Data) -> TokenStream {
    match data {
        Data::Struct(struct_data) => match &struct_data.fields {
            syn::Fields::Named(fields_named) => {
                let recurse = fields_named.named.iter().map(|f| {
                    let name = &f.ident;
                    let default = f
                        .attrs
                        .iter()
                        .find(|&attr| {
                            attr.meta.require_list().is_ok_and(|named| {
                                named.path.get_ident().is_some_and(|ident| {
                                    ident.to_string().as_str() == "config_default"
                                })
                            })
                        })
                        .map_or({
                            let ty = f.ty.clone();
                            quote!( #ty::default())
                        },|attr| {
                            let inner = attr.meta.require_list().unwrap();
                            let default_val: Expr = inner.parse_args().unwrap();
                            quote! {#default_val}
                        });
                    quote!( #name: {#default})
                });

                quote! {
                    #(#recurse,)*
                }
            }
            syn::Fields::Unnamed(_) => unimplemented!(),
            syn::Fields::Unit => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}
