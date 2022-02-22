#![warn(missing_docs)]
#![crate_name = "wwsvc_rs"]

//! # WEBSERVICES Client
//!
//! `wwsvc_rs` is a web client which is used to consume SoftENGINE's WEBSERVICES, a proprietary API for their software WEBWARE.

extern crate reqwest;
extern crate encoding_rs;
extern crate httpdate;
extern crate md5;
extern crate serde;
#[macro_use]
extern crate serde_json;

/// Module containing the pagination cursor.
pub mod cursor;
/// Module containing the app hash, which is needed for each request.
pub mod app_hash;

pub use reqwest::Method;
pub use serde_json::Value;
pub use cursor::Cursor;
pub use app_hash::AppHash;

/// Module containing the client.
pub mod client;
pub use client::WebwareClient;
pub use reqwest::Response;