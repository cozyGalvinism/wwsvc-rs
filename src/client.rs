use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Response;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use url::Url;

use crate::app_hash::AppHash;
use crate::client::states::*;
use crate::consts::{
    EXECUTE_MODE_SYNCHRON, HEADER_EXECUTE_MODE, HEADER_HASH, HEADER_REQUEST_ID,
    HEADER_RESULT_MAX_LINES, HEADER_RESULT_TYPE, HEADER_TIMESTAMP, RESULT_TYPE_BIN,
    RESULT_TYPE_JSON,
};
use crate::error::WWSVCError;
use crate::params::Parameters;
use crate::requests::{ExecJsonRequest, RequestToHttpString, ToServiceFunctionParameters};
use crate::responses::RegisterResponse;
use crate::{Credentials, WWClientResult};

/// Builder for constructing a [`WebwareClient`].
///
/// Create one via [`WebwareClient::builder`], set the required fields, then call
/// [`build`](WebwareClientBuilder::build).
///
/// Required fields: `webware_url`, `vendor_hash`, `app_hash`, `secret`, `revision`.
/// Calling [`build`](WebwareClientBuilder::build) without one of them returns
/// [`WWSVCError::MissingField`].
#[derive(Clone, Default)]
pub struct WebwareClientBuilder {
    webware_url: Option<String>,
    vendor_hash: Option<String>,
    app_hash: Option<String>,
    secret: Option<String>,
    revision: Option<u32>,
    credentials: Option<Credentials>,
    result_max_lines: Option<u32>,
    allow_insecure: bool,
    timeout: Option<std::time::Duration>,
    client: Option<reqwest::Client>,
}

impl WebwareClientBuilder {
    /// Full URL to the WEBWARE instance without the path to the WWSVC.
    ///
    /// Example: `https://localhost:8080`
    pub fn webware_url(mut self, webware_url: &str) -> Self {
        self.webware_url = Some(webware_url.to_string());
        self
    }

    /// Vendor hash of the application.
    pub fn vendor_hash(mut self, vendor_hash: &str) -> Self {
        self.vendor_hash = Some(vendor_hash.to_string());
        self
    }

    /// Application hash of the application.
    pub fn app_hash(mut self, app_hash: &str) -> Self {
        self.app_hash = Some(app_hash.to_string());
        self
    }

    /// Application secret, assigned by the WEBWARE instance.
    pub fn secret(mut self, secret: &str) -> Self {
        self.secret = Some(secret.to_string());
        self
    }

    /// Revision of the application.
    pub fn revision(mut self, revision: u32) -> Self {
        self.revision = Some(revision);
        self
    }

    /// Pre-existing credentials of the client.
    ///
    /// If set, [`WebwareClient::register`] reuses this service pass instead of
    /// performing a network `REGISTER`.
    pub fn credentials(mut self, credentials: Credentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    /// Maximum amount of objects that are returned in a request (default: 1000).
    pub fn result_max_lines(mut self, result_max_lines: u32) -> Self {
        self.result_max_lines = Some(result_max_lines);
        self
    }

    /// Allow unsafe SSL certificates (default: false).
    pub fn allow_insecure(mut self, allow_insecure: bool) -> Self {
        self.allow_insecure = allow_insecure;
        self
    }

    /// Timeout for requests (default: 60 seconds).
    ///
    /// Ignored if a custom client is set via [`reqwest_client`](Self::reqwest_client).
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Use a pre-configured [`reqwest::Client`] instead of letting the builder construct one.
    ///
    /// Use this as an escape hatch for advanced HTTP configuration (proxies, custom root
    /// certificates, certificate pinning, connection pools, ...). When set,
    /// [`allow_insecure`](Self::allow_insecure) and [`timeout`](Self::timeout) are ignored —
    /// configure those on the provided client directly.
    pub fn reqwest_client(mut self, client: reqwest::Client) -> Self {
        self.client = Some(client);
        self
    }

    /// Builds the [`WebwareClient`].
    ///
    /// # Errors
    ///
    /// - [`WWSVCError::MissingField`] if a required field (`webware_url`, `vendor_hash`,
    ///   `app_hash`, `secret`, `revision`) was not set.
    /// - [`WWSVCError::UrlParseError`] if `webware_url` is not a valid URL.
    /// - [`WWSVCError::ReqwestError`] if the underlying HTTP client cannot be built.
    pub fn build(self) -> WWClientResult<WebwareClient<Unregistered>> {
        let webware_url = self
            .webware_url
            .ok_or(WWSVCError::MissingField("webware_url"))?;
        let vendor_hash = self
            .vendor_hash
            .ok_or(WWSVCError::MissingField("vendor_hash"))?;
        let app_hash = self.app_hash.ok_or(WWSVCError::MissingField("app_hash"))?;
        let secret = self.secret.ok_or(WWSVCError::MissingField("secret"))?;
        let revision = self.revision.ok_or(WWSVCError::MissingField("revision"))?;

        let webware_url = Url::parse(&webware_url)?.join("/WWSVC/")?;

        let result_max_lines = self.result_max_lines.unwrap_or(1000);

        let client = match self.client {
            Some(client) => client,
            None => {
                let timeout = self
                    .timeout
                    .unwrap_or_else(|| std::time::Duration::from_secs(60));
                reqwest::Client::builder()
                    .danger_accept_invalid_certs(self.allow_insecure)
                    .timeout(timeout)
                    .build()?
            }
        };

        Ok(WebwareClient {
            webware_url,
            vendor_hash,
            app_hash,
            secret,
            revision,
            credentials: self.credentials,
            mutable_state: Arc::new(Mutex::new(MutableClientState {
                result_max_lines,
                current_request: 0,
            })),
            client,
            state: std::marker::PhantomData::<Unregistered>,
        })
    }
}

/// Contains the the states the client can be in
pub mod states {
    /// The state of the client
    ///
    /// Unregistered: The client is not registered
    #[derive(Clone)]
    pub struct Unregistered;
    /// The state of the client
    ///
    /// Registered: The client is registered
    #[derive(Clone)]
    pub struct Registered;

    /// Marker trait for a ready client
    pub trait Ready: Send + Sync {}

    impl Ready for Registered {}
}

/// Authentication values produced while building request headers.
///
/// Lets callers (e.g. `prepare_request`) reuse the computed hash and timestamp in the
/// request body without re-parsing them out of the `HeaderMap`.
struct AuthParts {
    /// The `WWSVC-HASH` value.
    hash: String,
    /// The `WWSVC-TS` value.
    timestamp: String,
    /// The request ID used for this request (matches the `WWSVC-REQID` header).
    request_id: u32,
}

/// Contains mutable state that requires interior mutability
#[derive(Debug)]
struct MutableClientState {
    /// Maximum amount of objects that are returned in a request
    result_max_lines: u32,
    /// Current request ID
    current_request: u32,
}

/// The web client to consume SoftENGINE's WEBSERVICES
///
/// **Note on `Clone`:** cloning shares the underlying mutable state (the request-id
/// counter and the pagination cursor) via `Arc<Mutex<...>>`. Clones are **not**
/// independent — a request made through one clone advances the request id and cursor
/// seen by all other clones. Build a fresh client with [`WebwareClient::builder`] if you
/// need isolated state.
#[derive(Clone)]
pub struct WebwareClient<State = Unregistered> {
    /// Full URL to the WEBWARE instance
    webware_url: Url,
    /// Vendor hash of the application
    vendor_hash: String,
    /// Application hash of the application
    app_hash: String,
    /// Application secret, assigned by the WEBWARE instance
    secret: String,
    /// Revision of the application
    revision: u32,
    /// Credentials of the client
    credentials: Option<Credentials>,
    /// Mutable state protected by a mutex for interior mutability
    mutable_state: Arc<Mutex<MutableClientState>>,
    /// The client
    client: reqwest::Client,

    state: std::marker::PhantomData<State>,
}

impl WebwareClient {
    /// Creates a builder for the client
    pub fn builder() -> WebwareClientBuilder {
        WebwareClientBuilder::default()
    }

    /// Sends a `REGISTER` request to the WEBWARE instance and returns a registered client
    /// or an error.
    ///
    /// **Note:** if the client was built with pre-existing [`Credentials`] (via
    /// [`WebwareClientBuilder::credentials`]), this method returns a [`Registered`] client
    /// **without** performing a network `REGISTER` — it reuses the existing service pass.
    /// A stale or invalid pass is therefore only detected on the first real request.
    pub async fn register(self) -> WWClientResult<WebwareClient<Registered>> {
        if self.credentials.is_some() {
            return Ok(WebwareClient {
                webware_url: self.webware_url,
                vendor_hash: self.vendor_hash,
                app_hash: self.app_hash,
                secret: self.secret,
                revision: self.revision,
                credentials: self.credentials,
                mutable_state: self.mutable_state,
                client: self.client,
                state: std::marker::PhantomData::<Registered>,
            });
        }

        // join self.webware_url and the register path
        // example: "WWSERVICE", "REGISTER", &self.vendor_hash, &self.app_hash, &self.secret, &self.revision.to_string()
        let target_url = self
            .webware_url
            .join("WWSERVICE/")?
            .join("REGISTER/")?
            .join(&format!("{}/", self.vendor_hash))?
            .join(&format!("{}/", self.app_hash))?
            .join(&format!("{}/", self.secret))?
            .join(&format!("{}/", self.revision))?;
        let response = self.client.get(target_url).send().await?;
        let response_obj = response.json::<RegisterResponse>().await?;

        Ok(WebwareClient {
            webware_url: self.webware_url,
            vendor_hash: self.vendor_hash,
            app_hash: self.app_hash,
            secret: self.secret,
            revision: self.revision,
            credentials: Some(Credentials {
                service_pass: response_obj.service_pass.pass_id,
                app_id: response_obj.service_pass.app_id,
            }),
            mutable_state: self.mutable_state,
            client: self.client,
            state: std::marker::PhantomData::<Registered>,
        })
    }

    /// Provides a harness for operating with the client by registering, running the provided closure and then deregistering
    /// the client.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = wwsvc_rs::WebwareClient::builder()
    ///            .webware_url("https://meine-webware.de")
    ///            .vendor_hash("my-vendor-hash")
    ///            .app_hash("my-app-hash")
    ///            .secret("1")
    ///            .revision(1)
    ///            .build()
    ///            .expect("failed to build client");
    ///
    ///     let article_result = client
    ///         .with_registered(async |registered_client| {
    ///             // Do something with the registered client
    ///         })
    ///         .await;
    /// }
    /// ```
    pub async fn with_registered<F, T>(self, f: F) -> WWClientResult<T>
    where
        F: AsyncFnOnce(&WebwareClient<Registered>) -> T,
    {
        let client = self.register().await?;
        let result = f(&client).await;
        client.deregister().await?;
        Ok(result)
    }
}

impl<State: Ready> WebwareClient<State> {
    /// Returns the credentials of the current client.
    ///
    /// # Panics
    ///
    /// Panics if the client has no credentials. Use [`try_credentials`](Self::try_credentials)
    /// for a non-panicking variant.
    pub fn credentials(&self) -> &Credentials {
        self.credentials.as_ref().unwrap()
    }

    /// Returns the credentials of the current client, if any.
    pub fn try_credentials(&self) -> Option<&Credentials> {
        self.credentials.as_ref()
    }

    /// Sets the maximum amount of results that are returned in a response
    pub async fn set_result_max_lines(&self, max_lines: u32) {
        let mut state = self.mutable_state.lock().await;
        state.result_max_lines = max_lines;
    }

    /// Returns a set of headers, that are required on all requests to the WEBSERVICES (except `REGISTER`).
    ///
    /// This will automatically append necessary authentication headers and increase the request ID, if `register()` was successful.
    pub async fn get_default_headers(
        &self,
        additional_headers: Option<HashMap<&str, &str>>,
    ) -> WWClientResult<HeaderMap> {
        Ok(self.build_headers(additional_headers).await?.0)
    }

    /// Builds the default headers and, when the client is authenticated, returns the auth
    /// values (`WWSVC-HASH` and `WWSVC-TS`) alongside them so callers can reuse them in the
    /// request body without re-parsing the `HeaderMap`.
    ///
    /// This advances the request ID exactly once.
    async fn build_headers(
        &self,
        additional_headers: Option<HashMap<&str, &str>>,
    ) -> WWClientResult<(HeaderMap, Option<AuthParts>)> {
        let mut state = self.mutable_state.lock().await;
        let max_lines = state.result_max_lines;

        let mut headers = HashMap::new();
        let mut auth = None;

        if let Some(credentials) = &self.credentials {
            let app_hash = AppHash::new(state.current_request, &credentials.app_id);
            state.current_request = app_hash.request_id;
            let request_id = state.current_request;

            let hash = format!("{:x}", app_hash);
            let timestamp = app_hash.date_formatted.clone();

            headers.insert(HEADER_REQUEST_ID.to_string(), request_id.to_string());
            headers.insert(HEADER_TIMESTAMP.to_string(), timestamp.clone());
            headers.insert(HEADER_HASH.to_string(), hash.clone());

            auth = Some(AuthParts {
                hash,
                timestamp,
                request_id,
            });
        }

        headers.insert(
            HEADER_EXECUTE_MODE.to_string(),
            EXECUTE_MODE_SYNCHRON.to_string(),
        );
        headers.insert(HEADER_RESULT_TYPE.to_string(), RESULT_TYPE_JSON.to_string());
        headers.insert(HEADER_RESULT_MAX_LINES.to_string(), max_lines.to_string());

        if let Some(additional_headers) = additional_headers {
            headers.extend(
                additional_headers
                    .iter()
                    .map(|(s1, s2)| (s1.to_string(), s2.to_string())),
            );
        }

        let header_map: HeaderMap = (&headers)
            .try_into()
            .map_err(|_| WWSVCError::InvalidHeader)?;
        Ok((header_map, auth))
    }

    /// Returns the same set of headers, that `get_default_headers()` returns, except the result type header is set to `BIN` instead.
    pub async fn get_bin_headers(
        &self,
        additional_headers: Option<HashMap<&str, &str>>,
    ) -> WWClientResult<HeaderMap> {
        let mut headers = self.get_default_headers(additional_headers).await?;
        headers.remove(HEADER_RESULT_TYPE);
        headers.append(HEADER_RESULT_TYPE, HeaderValue::from_str(RESULT_TYPE_BIN)?);
        Ok(headers)
    }

    /// Sends a `DEREGISTER` request to the WEBWARE instance, in order to invalidate the service pass.
    pub async fn deregister(self) -> WWClientResult<WebwareClient<Unregistered>> {
        if let Some(credentials) = &self.credentials {
            let target_url = self
                .webware_url
                .join("WWSERVICE/")?
                .join("DEREGISTER/")?
                .join(&format!("{}/", &credentials.service_pass))?;
            let headers = self.get_default_headers(None).await?;
            let _ = self.client.get(target_url).headers(headers).send().await;
        }

        Ok(WebwareClient {
            webware_url: self.webware_url,
            vendor_hash: self.vendor_hash,
            app_hash: self.app_hash,
            secret: self.secret,
            revision: self.revision,
            credentials: None,
            mutable_state: self.mutable_state,
            client: self.client,
            state: std::marker::PhantomData::<Unregistered>,
        })
    }

    /// Prepares a request to the WEBSERVICES.
    ///
    /// This will return a `[reqwest::Request]` object, that can be executed using the `execute_request` method.
    ///
    /// **NOTE:** This method advances the internal request ID.
    pub async fn prepare_request(
        &self,
        method: reqwest::Method,
        function: &str,
        version: u32,
        parameters: Parameters,
        additional_headers: Option<HashMap<&str, &str>>,
    ) -> WWClientResult<reqwest::Request> {
        if self.credentials.is_none() {
            return Err(WWSVCError::NotAuthenticated);
        }

        let target_url = self.webware_url.join("EXECJSON")?;
        let (headers, auth) = self.build_headers(additional_headers).await?;
        let auth = auth.ok_or(WWSVCError::NotAuthenticated)?;

        let parameters = parameters.to_service_function_parameters();

        let body = ExecJsonRequest::new(
            function,
            parameters,
            version,
            &self.credentials.as_ref().unwrap().service_pass,
            &auth.hash,
            &auth.timestamp,
            auth.request_id,
        );

        let request = self
            .client
            .request(method, target_url)
            .headers(headers)
            .json(&body)
            .build()?;

        Ok(request)
    }

    /// Executes a prepared request to the WEBSERVICES and returns the response object.
    pub async fn execute_request(&self, request: reqwest::Request) -> WWClientResult<Response> {
        Ok(self.client.execute(request).await?)
    }

    /// Performs a request to the WEBSERVICES and returns a JSON value.
    pub async fn request(
        &self,
        method: reqwest::Method,
        function: &str,
        version: u32,
        parameters: Parameters,
        additional_headers: Option<HashMap<&str, &str>>,
    ) -> WWClientResult<serde_json::Value> {
        self.request_generic::<serde_json::Value>(
            method,
            function,
            version,
            parameters,
            additional_headers,
        )
        .await
    }

    /// Performs a request to the WEBSERVICES and returns a response object.
    pub async fn request_as_response(
        &self,
        method: reqwest::Method,
        function: &str,
        version: u32,
        parameters: Parameters,
        additional_headers: Option<HashMap<&str, &str>>,
    ) -> WWClientResult<Response> {
        let request = self
            .prepare_request(method, function, version, parameters, additional_headers)
            .await?;
        tracing::debug!(
            request = request.to_http_string().unwrap_or_default(),
            "send request"
        );
        self.execute_request(request).await
    }

    /// Performs a request to the WEBSERVICES and deserializes the response to the type `T`.
    ///
    /// This is the method most callers want. The request methods form a layered stack —
    /// pick the lowest level you need:
    ///
    /// - [`request_generic`](Self::request_generic) — deserialize the response into `T` (this method).
    /// - [`request`](Self::request) — same, but returns a raw [`serde_json::Value`]; use when
    ///   deserialization into a concrete type fails due to structural quirks.
    /// - [`request_as_response`](Self::request_as_response) — returns the raw [`Response`]
    ///   (e.g. for binary results); you handle the body.
    /// - [`prepare_request`](Self::prepare_request) + [`execute_request`](Self::execute_request)
    ///   — split request building from execution for full control.
    pub async fn request_generic<T>(
        &self,
        method: reqwest::Method,
        function: &str,
        version: u32,
        parameters: Parameters,
        additional_headers: Option<HashMap<&str, &str>>,
    ) -> WWClientResult<T>
    where
        T: DeserializeOwned,
    {
        let response = self
            .request_as_response(method, function, version, parameters, additional_headers)
            .await?;
        let response_obj = response.json::<T>().await?;
        Ok(response_obj)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_succeeds_with_all_required_fields() {
        let client = WebwareClient::builder()
            .webware_url("https://example.com")
            .vendor_hash("vendor")
            .app_hash("app")
            .secret("1")
            .revision(1)
            .build();
        assert!(client.is_ok());
    }

    #[test]
    fn build_fails_on_missing_required_field() {
        let result = WebwareClient::builder()
            .webware_url("https://example.com")
            .vendor_hash("vendor")
            .app_hash("app")
            .secret("1")
            // revision intentionally omitted
            .build();
        assert!(matches!(result, Err(WWSVCError::MissingField("revision"))));
    }

    #[test]
    fn build_fails_on_invalid_url() {
        let result = WebwareClient::builder()
            .webware_url("not a url")
            .vendor_hash("vendor")
            .app_hash("app")
            .secret("1")
            .revision(1)
            .build();
        assert!(matches!(result, Err(WWSVCError::UrlParseError(_))));
    }
}
