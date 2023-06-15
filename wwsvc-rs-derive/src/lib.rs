#![warn(missing_docs)]
#![crate_name = "wwsvc_rs_derive"]
//! # wwsvc-rs-derive
//!
//! This is a set of macros to derive the traits from wwsvc-rs.

extern crate proc_macro;

use darling::{FromDeriveInput, FromField};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(FromDeriveInput)]
#[darling(attributes(wwsvc))]
struct WWSVCGetAttributes {
    function: String,
    #[darling(default)]
    version: Option<u32>,
}

#[derive(FromField)]
#[darling(attributes(serde), allow_unknown_fields)]
struct WWSVCGetFieldAttributes {
    rename: String,
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
    let WWSVCGetAttributes { function, version } =
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
    let function_list = format!("{}LISTE", function);
    let full_function_name = format!("{function}.GET");
    let response_ident = syn::Ident::new(&response_type, name.span());
    let container_ident = syn::Ident::new(&container_type, name.span());
    // collect fields to comma separated string
    let available_fields = fields.join(",");

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
            pub container: #container_ident,
        }

        /// Container struct for the list of items.
        #[derive(serde::Deserialize, Debug, Clone)]
        pub struct #container_ident {
            /// The list of items.
            #[serde(rename = #function)]
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
    };

    gen.into()
}
