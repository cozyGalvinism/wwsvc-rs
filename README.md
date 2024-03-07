# wwsvc-rs

[![crate-badge]][crate-link] [![docs-badge]][docs-link]

[crate-badge]: https://img.shields.io/crates/v/wwsvc-rs.svg
[crate-link]: https://crates.io/crates/wwsvc-rs
[docs-badge]: https://docs.rs/wwsvc-rs/badge.svg
[docs-link]: https://docs.rs/wwsvc-rs

A web client which is used to consume SoftENGINE's WEBSERVICES, a proprietary API for their ERPSuite.

## How to use

Here is an example using the `derive` feature, which is the preferred way
of using this crate.

```rust
use wwsvc_rs::{WebwareClient, Unregistered, WWSVCGetData, collection};

#[derive(WWSVCGetData, Debug, Clone, serde::Deserialize)]
#[wwsvc(function = "ARTIKEL")]
pub struct ArticleData {
    #[serde(rename = "ART_1_25")]
    pub article_number: String
}

#[tokio::main]
async fn main() {
    let client = WebwareClient::builder()
        .webware_url("https://meine-webware.de")
        .vendor_hash("my-vendor-hash")
        .app_hash("my-app-hash")
        .secret("1")
        .revision(1)
        .build();
    let mut registered_client = client.register().await.expect("failed to register");
    let articles = ArticleData::get(&mut registered_client, collection! {
        "ARTNR" => "Artikel19Prozent",
    }).await;
    println!("{:#?}", articles);

    registered_client.deregister().await.unwrap();
}
```

You can, however, also define your own data structures
to use and reuse. For these purposes, you can directly use the client:

```rust
use reqwest::Method;
use wwsvc_rs::{collection, WWSVCGetData, generate_get_response};

#[derive(Debug, serde::Deserialize, Clone)]
pub struct ArticleData {
    #[serde(rename = "ART_1_25")]
    pub article_number: String,
}

// You don't have to use this macro, it does however make generating responses a lot easier.
generate_get_response!(ArticleResponse, "ARTIKELLISTE", ArticleContainer, "ARTIKEL");

#[tokio::main]
async fn main() {
    let client = WebwareClient::builder()
        .webware_url("https://meine-webware.de")
        .vendor_hash("my-vendor-hash")
        .app_hash("my-app-hash")
        .secret("1")
        .revision(1)
        .build();
    let mut registered_client = client.register().await.expect("failed to register");

    let articles = registered_client.request_generic::<ArticleResponse<ArticleData>>(Method::PUT, "ARTIKEL.GET", 1, collection! {
        "ARTNR" => "Artikel19Prozent",
    }, None)
        .await
        .unwrap();

    println!("{:#?}", articles.container.list.unwrap());

    registered_client.deregister().await.unwrap();
}

```

## Safety

This project uses `#![forbid(unsafe_code)]` to ensure everything is implemented in 100% safe Rust.

## Versioning

This project adheres to [semantic versioning](https://semver.org/).
