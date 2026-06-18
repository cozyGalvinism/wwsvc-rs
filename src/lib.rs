#![warn(missing_docs)]
#![forbid(unsafe_code)]
#![crate_name = "wwsvc_rs"]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! # WEBSERVICES Client
//!
//! `wwsvc_rs` is a web client which is used to consume SoftENGINE's WEBSERVICES, a proprietary API for their software WEBWARE.
//!
//! ## Usage
//!
//! Here is an example using the `derive` feature, which is the preferred way
//! of using this crate.
//!
//! ```rust,no_run
//! use wwsvc_rs::{WebwareClient, WWSVCGetData, collection};
//!
//! #[derive(WWSVCGetData, Debug, Clone, serde::Deserialize)]
//! #[wwsvc(function = "ARTIKEL")]
//! pub struct ArticleData {
//!     #[serde(rename = "ART_1_25")]
//!     pub article_number: String
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = WebwareClient::builder()
//!         .webware_url("https://meine-webware.de")
//!         .vendor_hash("my-vendor-hash")
//!         .app_hash("my-app-hash")
//!         .secret("1")
//!         .revision(1)
//!         .build()
//!         .expect("failed to build client");
//!     let registered_client = client.register().await.expect("failed to register");
//!     let articles = ArticleData::get(&registered_client, collection! {
//!         "ARTNR" => "Artikel19Prozent",
//!     }).await;
//!     println!("{:#?}", articles);
//!
//!     registered_client.deregister().await.unwrap();
//! }
//! ```
//!
//! You can, however, also define your own data structures
//! to use and reuse. For these purposes, you can directly use the client:
//!
//! ```rust,no_run
//! use reqwest::Method;
//! use wwsvc_rs::{collection, WebwareClient, WWSVCGetData, generate_get_response};
//!
//! #[derive(Debug, serde::Deserialize, Clone)]
//! pub struct ArticleData {
//!     #[serde(rename = "ART_1_25")]
//!     pub article_number: String,
//! }
//!
//! // You don't have to use this macro, it does however make generating responses a lot easier.
//! generate_get_response!(ArticleResponse, "ARTIKELLISTE", ArticleContainer, "ARTIKEL");
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = WebwareClient::builder()
//!         .webware_url("https://meine-webware.de")
//!         .vendor_hash("my-vendor-hash")
//!         .app_hash("my-app-hash")
//!         .secret("1")
//!         .revision(1)
//!         .build()
//!         .expect("failed to build client");
//!     let registered_client = client.register().await.expect("failed to register");
//!
//!     let articles = registered_client.request_generic::<ArticleResponse<ArticleData>>(Method::PUT, "ARTIKEL.GET", 1, collection! {
//!         "ARTNR" => "Artikel19Prozent",
//!     }, None)
//!         .await
//!         .unwrap();
//!
//!     println!("{:#?}", articles.container.unwrap().list.unwrap());
//!
//!     registered_client.deregister().await.unwrap();
//! }
//!
//! ```

extern crate self as wwsvc_rs;

/// Internal app hash, computed for each request.
pub(crate) mod app_hash;
/// Internal pagination cursor.
pub(crate) mod cursor;
/// Internal protocol constants.
pub(crate) mod consts;
/// Module containing the error type.
pub mod error;
/// Module containing the macros.
pub mod macros;
/// Module containing cursor response handling.
pub mod cursor_response;
/// Module containing the structure for parameters.
pub mod params;
/// Module containing requests.
pub mod requests;
/// Module containing the [`traits::WWSVCGetData`] trait.
///
/// Only available with the `derive` feature, which provides the derive macro that
/// implements this trait (and the `async_trait` re-export it relies on).
#[cfg(feature = "derive")]
pub mod traits;

mod credentials;
/// Module containing common response types.
pub mod responses;

pub use cursor_response::{
    CursoredRequests, CursoredResponse, HasComResult, HasList, ItemsWithComResult,
};
pub use futures;
pub use params::Parameters;
pub use reqwest::Method;
pub use serde_json::Value;

#[cfg(feature = "derive")]
pub use async_trait::async_trait;
#[cfg(feature = "derive")]
pub use traits::WWSVCGetData;
#[cfg(feature = "derive")]
pub use wwsvc_rs_derive::WWSVCGetData;

/// Module containing the client.
pub mod client;
pub use client::states::*;
pub use client::WebwareClient;
pub use credentials::Credentials;
pub use error::WWSVCError;
pub use reqwest::Response;

/// Result type for the wwsvc-rs crate.
pub type WWClientResult<T> = std::result::Result<T, error::WWSVCError>;
