#![warn(missing_docs)]
#![crate_name = "wwsvc_rs_derive"]
//! # wwsvc-rs-derive
//!
//! This is a set of macros to derive the traits from wwsvc-rs.

extern crate proc_macro;

use darling::{FromDeriveInput, FromField, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(FromDeriveInput)]
#[darling(attributes(wwsvc))]
struct WWSVCGetAttributes {
    function: String,
    #[darling(default)]
    version: Option<u32>,
    #[darling(default)]
    list_name: Option<String>,
    #[darling(default)]
    container_name: Option<String>,
}

struct RenameField(String);

impl FromMeta for RenameField {
    fn from_string(value: &str) -> darling::Result<Self> {
        Ok(RenameField(value.to_string()))
    }

    fn from_list(items: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
        let mut rename = None;
        for item in items {
            if let darling::ast::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue {
                path,
                value:
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }),
                ..
            })) = item
            {
                if path.is_ident("deserialize") {
                    rename = Some(lit_str.value());
                }
            }
        }
        if let Some(rename) = rename {
            Ok(RenameField(rename))
        } else {
            Err(darling::Error::custom(
                "serde(rename) requires a deserialize rename",
            ))
        }
    }
}

#[derive(FromField)]
#[darling(attributes(serde), allow_unknown_fields)]
struct WWSVCGetFieldAttributes {
    rename: RenameField,
}

/// Generates a response and a container struct based on the name of the struct and the function name.
///
/// ## Example
/// ```
/// use wwsvc_rs_proc::WWSVCGetData;
///
/// #[derive(WWSVCGetData, serde::Deserialize, Clone)]
/// #[wwsvc(function = "IDBID0026")]
/// pub struct TrackingData {
///     #[serde(rename = "IDB_0_20")]
///     pub index: String
/// }
/// ```
#[proc_macro_derive(WWSVCGetData, attributes(wwsvc))]
pub fn wwsvc_wrapper_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let name = &ast.ident;
    let WWSVCGetAttributes { function, version, list_name, container_name } =
        WWSVCGetAttributes::from_derive_input(&ast).unwrap();

    // parse fields and add #[serde(rename = "#name")] to each field
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { named: fields, .. }),
        ..
    }) = &ast.data
    {
        fields
            .iter()
            .map(|field| {
                let WWSVCGetFieldAttributes { rename } = WWSVCGetFieldAttributes::from_field(field)
                    .expect("WWSVCGetData requires serde renames!");
                rename
            })
            .collect::<Vec<_>>()
    } else {
        panic!("WWSVCGetData can only be derived for structs with named fields.");
    };

    let response_type = format!("{}Response", name);
    let container_type = format!("{}Container", name);
    let function_list = match list_name {
        Some(name) => name,
        None => format!("{}LISTE", function),
    };
    let container = match container_name {
        Some(name) => name,
        None => function.clone(),
    };
    let full_function_name = format!("{function}.GET");
    let response_ident = syn::Ident::new(&response_type, name.span());
    let container_ident = syn::Ident::new(&container_type, name.span());
    // collect fields to comma separated string
    let available_fields = fields
        .into_iter()
        .map(|field| field.0)
        .collect::<Vec<_>>()
        .join(",");

    let function_version = if let Some(version) = version {
        quote! {
            const VERSION: u32 = #version;
        }
    } else {
        quote! {}
    };

    let gen = quote! {
        /// A response struct for a WWSVC GET request.
        #[derive(serde::Deserialize, Debug, Clone)]
        pub struct #response_ident {
            /// The COMRESULT of the request. Contains information about the status of the request.
            #[serde(rename = "COMRESULT")]
            pub com_result: wwsvc_rs::responses::ComResult,
            /// The container struct for the list of items.
            #[serde(rename = #function_list)]
            pub container: Option<#container_ident>,
        }

        /// Container struct for the list of items.
        #[derive(serde::Deserialize, Debug, Clone)]
        pub struct #container_ident {
            /// The list of items.
            #[serde(rename = #container)]
            pub list: Option<Vec<#name>>,
        }

        #[wwsvc_rs::async_trait]
        impl wwsvc_rs::traits::WWSVCGetData for #name {
            const FUNCTION: &'static str = #full_function_name;
            #function_version
            const FIELDS: &'static str = #available_fields;

            type Response = #response_ident;
            type Container = #container_ident;
        }

        impl wwsvc_rs::cursor_response::HasList<#name> for #response_ident {
            fn into_items(self) -> Option<Vec<#name>> {
                if let Some(container) = self.container {
                    container.list
                } else {
                    None
                }
            }
        }

        impl wwsvc_rs::cursor_response::HasComResult for #response_ident {
            fn comresult(&self) -> &wwsvc_rs::responses::ComResult {
                &self.com_result
            }
        }
    };

    gen.into()
}
