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

#[cfg(feature = "default")]
pub mod client;
#[cfg(feature = "default")]
pub use client::{WebwareClient, AppHash};

#[cfg(feature = "async")]
pub mod async_client;
#[cfg(feature = "async")]
pub use async_client as _async;