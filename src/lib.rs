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

pub mod cursor;

pub use reqwest::Method;
pub use serde_json::Value;
#[cfg(feature = "default")]
pub mod client;
#[cfg(feature = "default")]
pub use client::{WebwareClient, AppHash};
#[cfg(feature = "default")]
pub use reqwest::blocking::Response;

#[cfg(feature = "async")]
pub mod async_client;
#[cfg(feature = "async")]
pub use async_client as _async;
#[cfg(feature = "async")]
pub use reqwest::Response as AsyncResponse;