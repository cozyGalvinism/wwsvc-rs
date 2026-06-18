# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [5.0.0]

This is a breaking release focused on API usability, correctness, and a smaller public
surface.

### Added

- `WebwareClientBuilder::reqwest_client` — escape hatch to supply a pre-configured
  `reqwest::Client` (proxies, custom root certificates, certificate pinning, connection
  pools, ...). When set, `allow_insecure` and `timeout` are ignored.
- `WebwareClient::try_credentials` — non-panicking counterpart to `credentials()`.
- `builtin-responses` feature flag — gates the ready-made SoftENGINE response types
  (`ArtikelGetResponse`, `AdresseGetResponse`, ...). Off by default.
- `ItemsWithComResult` and `HasComResult` are now re-exported from the crate root.
- `WWSVCError::MissingField(&'static str)` for builder validation failures.

### Changed

- **`WebwareClient::builder()` returns a hand-written `WebwareClientBuilder`** instead of a
  `typed_builder`-generated one, and **`build()` now returns
  `WWClientResult<WebwareClient<Unregistered>>`** instead of panicking on a bad URL or HTTP
  client construction. Missing required fields return `Err(WWSVCError::MissingField(..))`.
- **Pagination cursors now live in `CursoredResponse`, not in the client.** You can run
  multiple independent cursored requests against the same client. The cursor id is sent and
  read back per request.
- `WWSVCGetData::get` takes `&WebwareClient<impl Ready>` (the redundant `+ Send` bound was
  removed; `Ready` already requires it).
- The minimum supported Rust version is now **1.85** (async closures).
- The `derive` feature now gates the `traits` module (`WWSVCGetData`). Previously the crate
  failed to compile with default features because the trait depended on the `async_trait`
  re-export, which is only present with `derive`.

### Removed

- **Client-side cursor methods**: `create_cursor`, `close_cursor`, `has_cursor`,
  `cursor_closed`, `suspend_cursor`, `resume_cursor`. Use a `CursoredResponse` (via
  `cursored_request` / `WWSVCGetData::get_cursored`) instead.
- The `CursoredRequests` impl for bare `WebwareClient<Registered>` (it always returned a
  misleading `NotAuthenticated` error). Calling `cursored_request` now requires
  `Arc<WebwareClient<Registered>>` — enforced at compile time.
- Public visibility of protocol internals: `AppHash`, `Cursor`, `ExecJsonRequest`,
  `ServiceFunction`, `ServiceFunctionParameter`, `ServicePassInfo`,
  `ToServiceFunctionParameters`, `RegisterResponse`, and `ServicePass` are now `pub(crate)`.
  `RequestToHttpString` and `responses::ComResult` remain public.

### Fixed

- **Request-id race**: the `WWSVC-REQID` header and the body `REQUESTID` are now guaranteed
  to match — both are derived from a single locked read instead of two.
- Builder no longer panics on an invalid URL or when the underlying HTTP client cannot be
  built; both are reported as errors.
- The derive macro now emits proper compiler diagnostics instead of panicking on misuse
  (non-struct targets, fields missing `#[serde(rename = ...)]`).
- `CursoredResponse::next_with_comresult` no longer clones the item list it already owns.

### Migration

```rust
// build() is now fallible
let client = WebwareClient::builder()
    .webware_url("https://example.de")
    .vendor_hash("…").app_hash("…").secret("…").revision(1)
    .build()?;                                  // was: .build();

// cursored requests require Arc
let client = Arc::new(client.register().await?);
let mut pages = client
    .cursored_request::<Article, ArticleResponse<Article>>(Method::PUT, "ARTIKEL.GET", 1, params, 100)
    .await?;
while let Some(batch) = pages.next().await? { /* … */ }

// client-side cursor toggles are gone — drive pagination through CursoredResponse instead

// opt back into the built-in SoftENGINE response types:
// wwsvc-rs = { version = "5", features = ["builtin-responses"] }
```
