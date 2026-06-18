use futures::future::BoxFuture;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Response;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::sync::Arc;
use tokio::sync::Mutex;
use typed_builder::TypedBuilder;
use url::Url;

use crate::client::states::*;
use crate::error::WWSVCError;
use crate::params::Parameters;
use crate::requests::{ExecJsonRequest, RequestToHttpString, ToServiceFunctionParameters};
use crate::responses::RegisterResponse;
use crate::{AppHash, Credentials, Cursor, WWClientResult};

/// The internal builder for constructing a `WebwareClient`
#[derive(TypedBuilder)]
#[builder(build_method(into = WebwareClient::<Unregistered>))]
pub struct InternalWebwareClient {
    /// Full URL to the WEBWARE instance without the path to the WWSVC
    ///
    /// Example: `https://localhost:8080`
    #[builder(setter(transform = |url: &str| {
        Url::parse(url).expect("Failed to parse URL").join("/WWSVC/").expect("Failed to join URL")
    }))]
    webware_url: Url,
    /// Vendor hash of the application
    #[builder(setter(transform = |vendor_hash: &str| vendor_hash.to_string()))]
    vendor_hash: String,
    /// Application hash of the application
    #[builder(setter(transform = |app_hash: &str| app_hash.to_string()))]
    app_hash: String,
    /// Application secret, assigned by the WEBWARE instance
    #[builder(setter(transform = |app_secret: &str| app_secret.to_string()))]
    secret: String,
    /// Revision of the application
    revision: u32,
    /// Credentials of the client
    #[builder(default, setter(transform = |credentials: Credentials| Some(credentials)))]
    credentials: Option<Credentials>,
    /// Maximum amount of objects that are returned in a request
    #[builder(default = 1000)]
    result_max_lines: u32,
    /// Allow unsafe SSL certificates
    #[builder(default = false)]
    allow_insecure: bool,
    /// Timeout for the request
    #[builder(default = std::time::Duration::from_secs(60))]
    timeout: std::time::Duration,
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

/// Contains mutable state that requires interior mutability
#[derive(Debug)]
struct MutableClientState {
    /// Maximum amount of objects that are returned in a request
    result_max_lines: u32,
    /// Request cursor for pagination,
    cursor: Option<Cursor>,
    /// Current request ID
    current_request: u32,
    /// Suspend the cursor
    suspend_cursor: bool,
}

/// The web client to consume SoftENGINE's WEBSERVICES
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

impl From<InternalWebwareClient> for WebwareClient<Unregistered> {
    fn from(client: InternalWebwareClient) -> Self {
        let req_client = reqwest::Client::builder()
            .danger_accept_invalid_certs(client.allow_insecure)
            .timeout(client.timeout)
            .build()
            .expect("Failed to build client");

        WebwareClient {
            webware_url: client.webware_url,
            vendor_hash: client.vendor_hash,
            app_hash: client.app_hash,
            secret: client.secret,
            revision: client.revision,
            credentials: client.credentials,
            mutable_state: Arc::new(Mutex::new(MutableClientState {
                result_max_lines: client.result_max_lines,
                cursor: None,
                current_request: 0,
                suspend_cursor: false,
            })),
            client: req_client,
            state: std::marker::PhantomData::<Unregistered>,
        }
    }
}

impl TryFrom<InternalWebwareClient> for WebwareClient<Registered> {
    type Error = WWSVCError;

    fn try_from(client: InternalWebwareClient) -> Result<Self, Self::Error> {
        let req_client = reqwest::Client::builder()
            .danger_accept_invalid_certs(client.allow_insecure)
            .timeout(client.timeout)
            .build()
            .expect("Failed to build client");

        if client.credentials.is_none() {
            return Err(WWSVCError::MissingCredentials);
        }

        Ok(WebwareClient {
            webware_url: client.webware_url,
            vendor_hash: client.vendor_hash,
            app_hash: client.app_hash,
            secret: client.secret,
            revision: client.revision,
            credentials: client.credentials,
            mutable_state: Arc::new(Mutex::new(MutableClientState {
                result_max_lines: client.result_max_lines,
                cursor: None,
                current_request: 0,
                suspend_cursor: false,
            })),
            client: req_client,
            state: std::marker::PhantomData::<Registered>,
        })
    }
}

impl WebwareClient {
    /// Creates a builder for the client
    pub fn builder() -> InternalWebwareClientBuilder {
        InternalWebwareClient::builder()
    }

    /// Sends a `REGISTER` request to the WEBWARE instance and returns a registered client
    /// or an error
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
    /// use wwsvc_rs::futures::FutureExt;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = wwsvc_rs::WebwareClient::builder()
    ///            .webware_url("https://meine-webware.de")
    ///            .vendor_hash("my-vendor-hash")
    ///            .app_hash("my-app-hash")
    ///            .secret("1")
    ///            .revision(1)
    ///            .build();
    ///
    ///     let article_result = client
    ///         .with_registered(|registered_client| async {
    ///             // Do something with the registered client
    ///         }.boxed())
    ///         .await;
    /// }
    /// ```
    pub async fn with_registered<F, T>(self, f: F) -> WWClientResult<T>
    where
        F: for<'a> FnOnce(&'a WebwareClient<Registered>) -> BoxFuture<'a, T>,
    {
        let client = self.register().await?;
        let result = f(&client).await;
        let _ = client.deregister().await?;
        Ok(result)
    }
}

impl<State: Ready> WebwareClient<State> {
    /// Creates a new pagination cursor and makes it available for the next requests (until it is closed)
    pub async fn create_cursor(&self, max_lines: u32) {
        let cursor = Cursor::new(max_lines);
        let mut state = self.mutable_state.lock().await;
        state.cursor = Some(cursor);
        state.result_max_lines = max_lines;
    }

    /// Closes the current cursor, if any, and removes it from the client
    pub async fn close_cursor(&self) {
        let mut state = self.mutable_state.lock().await;
        state.cursor = None;
    }

    /// Returns whether the client currently has a cursor
    pub async fn has_cursor(&self) -> bool {
        let state = self.mutable_state.lock().await;
        state.cursor.is_some()
    }

    /// Generates a set of credentials from the current client.
    pub fn credentials(&self) -> &Credentials {
        self.credentials.as_ref().unwrap()
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
        let mut state = self.mutable_state.lock().await;
        let mut max_lines = state.result_max_lines;

        let mut headers = HashMap::new();
        
        if let Some(credentials) = &self.credentials {
            let app_hash = AppHash::new(state.current_request, &credentials.app_id);
            state.current_request = app_hash.request_id;
            
            headers.insert("WWSVC-REQID".to_string(), format!("{}", state.current_request));
            headers.insert("WWSVC-TS".to_string(), app_hash.date_formatted.to_string());
            headers.insert("WWSVC-HASH".to_string(), format!("{:x}", app_hash));
            
            if !state.suspend_cursor {
                if let Some(cursor) = &state.cursor {
                    if !Cursor::closed(cursor) {
                        headers.insert("WWSVC-CURSOR".to_string(), cursor.cursor_id.to_string());
                        max_lines = cursor.max_lines;
                    }
                }
            }
        }
        
        headers.insert("WWSVC-EXECUTE-MODE".to_string(), "SYNCHRON".to_string());
        headers.insert("WWSVC-ACCEPT-RESULT-TYPE".to_string(), "JSON".to_string());
        headers.insert("WWSVC-ACCEPT-RESULT-MAX-LINES".to_string(), max_lines.to_string());

        if let Some(additional_headers) = additional_headers {
            headers.extend(
                additional_headers
                    .iter()
                    .map(|(s1, s2)| (s1.to_string(), s2.to_string())),
            );
        }

        (&headers).try_into().map_err(|_| WWSVCError::InvalidHeader)
    }

    /// Returns the same set of headers, that `get_default_headers()` returns, except the result type header is set to `BIN` instead.
    pub async fn get_bin_headers(
        &self,
        additional_headers: Option<HashMap<&str, &str>>,
    ) -> WWClientResult<HeaderMap> {
        let mut headers = self.get_default_headers(additional_headers).await?;
        headers.remove("WWSVC-ACCEPT-RESULT-TYPE");
        headers.append("WWSVC-ACCEPT-RESULT-TYPE", HeaderValue::from_str("BIN")?);
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
    /// **NOTE:** This method will also update the internal state of the client, such as the request ID and cursor.
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
        let headers = self.get_default_headers(additional_headers).await?;
        let app_hash_header = headers.get("WWSVC-HASH");
        let timestamp_header = headers.get("WWSVC-TS");
        let app_hash: String = app_hash_header
            .unwrap_or(&HeaderValue::from_str("").unwrap())
            .to_str()
            .map_err(|_| WWSVCError::HeaderValueToStrError)?
            .to_string();
        let timestamp: String = timestamp_header
            .unwrap_or(&HeaderValue::from_str("").unwrap())
            .to_str()
            .map_err(|_| WWSVCError::HeaderValueToStrError)?
            .to_string();

        let parameters = parameters.to_service_function_parameters();
        let current_request = {
            let state = self.mutable_state.lock().await;
            state.current_request
        };

        let body = ExecJsonRequest::new(
            function,
            parameters,
            version,
            &self.credentials.as_ref().unwrap().service_pass,
            &app_hash,
            &timestamp,
            current_request,
        );

        let request = self
            .client
            .request(method, target_url)
            .headers(headers)
            .json(&body)
            .build()?;

        Ok(request)
    }

    /// Executes a prepared request to the WEBSERVICES.
    ///
    /// This will execute the prepared request and return a response object.
    ///
    /// **NOTE:** This method will also update the internal state of the client, such as the request ID and cursor.
    pub async fn execute_request(&self, request: reqwest::Request) -> WWClientResult<Response> {
        let response = self.client.execute(request).await?;

        let mut state = self.mutable_state.lock().await;
        if !state.suspend_cursor {
            if let Some(cursor) = &mut state.cursor {
                if !Cursor::closed(cursor) && response.headers().contains_key("WWSVC-CURSOR") {
                    cursor.set_cursor_id(
                        response
                            .headers()
                            .get("WWSVC-CURSOR")
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string(),
                    );
                }
            }
        }

        Ok(response)
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
        let request =
            self.prepare_request(method, function, version, parameters, additional_headers).await?;
        tracing::debug!(request = request.to_http_string().unwrap_or_default(), "send request");
        let response = self.client.execute(request).await?;

        let mut state = self.mutable_state.lock().await;
        if !state.suspend_cursor {
            if let Some(cursor) = &mut state.cursor {
                if !Cursor::closed(cursor) && response.headers().contains_key("WWSVC-CURSOR") {
                    cursor.set_cursor_id(
                        response
                            .headers()
                            .get("WWSVC-CURSOR")
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string(),
                    );
                }
            }
        }

        Ok(response)
    }

    /// Performs a request to the WEBSERVICES and deserializes the response to the type `T`.
    ///
    /// **NOTE:** Due to the nature of the WEBSERVICES, deserialization might fail due to structural issues. In that case, use `request()` instead.
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

    /// Suspends the cursor, so that it is not used for the next request
    pub async fn suspend_cursor(&self) {
        let mut state = self.mutable_state.lock().await;
        state.suspend_cursor = true;
    }

    /// Resumes the cursor, so that it is used for the next request
    pub async fn resume_cursor(&self) {
        let mut state = self.mutable_state.lock().await;
        state.suspend_cursor = false;
    }

    /// Returns whether the current cursor is closed.
    ///
    /// Returns true if no cursor exists or if the cursor is closed.
    pub async fn cursor_closed(&self) -> bool {
        let state = self.mutable_state.lock().await;
        state.cursor.as_ref().map_or(true, |c| Cursor::closed(c))
    }
}
