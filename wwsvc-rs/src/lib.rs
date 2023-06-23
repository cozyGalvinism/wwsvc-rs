#![warn(missing_docs)]
#![crate_name = "wwsvc_rs"]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! # WEBSERVICES Client
//!
//! `wwsvc_rs` is a web client which is used to consume SoftENGINE's WEBSERVICES, a proprietary API for their software WEBWARE.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use wwsvc_rs::{WebwareClient, Unregistered, WWSVCGetData, collection};
//!
//! #[derive(WWSVCGetData, Debug, serde::Deserialize)]
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
//!         .build();
//!     let mut registered_client = client.register().await.expect("failed to register");
//!     let articles = ArticleData::get(&mut registered_client, collection! {
//!         "ARTNR" => "1004208001",
//!     }).await;
//!     println!("{:#?}", articles);
//! }
//! ```

extern crate encoding_rs;
extern crate httpdate;
extern crate md5;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate self as wwsvc_rs;

/// Module containing the app hash, which is needed for each request.
pub mod app_hash;
/// Module containing the pagination cursor.
pub mod cursor;
/// Module containing the error type.
pub mod error;
/// Module containing the macros.
pub mod macros;
/// Module containing trais.
pub mod traits;

mod credentials;
/// Module containing common response types.
pub mod responses;

pub use app_hash::AppHash;
pub use cursor::Cursor;
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
pub use reqwest::Response;

/// Result type for the wwsvc-rs crate.
pub type WWClientResult<T> = std::result::Result<T, error::WWSVCError>;
