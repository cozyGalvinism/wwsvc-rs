use futures::future::BoxFuture;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Response;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use typed_builder::TypedBuilder;
use url::Url;

use crate::client::states::*;
use crate::error::WWSVCError;
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
    pub struct Unregistered;
    /// The state of the client
    ///
    /// Registered: The client is registered
    pub struct Registered;

    /// The state of the client
    ///
    /// Cursor: The client is registered and has a cursor
    pub struct OpenCursor;

    /// Marker trait for a ready client
    pub trait Ready {}

    impl Ready for Registered {}
    impl Ready for OpenCursor {}
}

/// The web client to consume SoftENGINE's WEBSERVICES
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
    /// Maximum amount of objects that are returned in a request
    result_max_lines: u32,
    /// Request cursor for pagination,
    cursor: Option<Cursor>,
    /// Current request ID
    current_request: u32,
    /// The client
    client: reqwest::Client,
    /// Suspend the cursor
    suspend_cursor: bool,

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
            credentials: None,
            result_max_lines: client.result_max_lines,
            cursor: None,
            current_request: 0,
            client: req_client,
            suspend_cursor: false,
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
            result_max_lines: client.result_max_lines,
            cursor: None,
            current_request: 0,
            client: req_client,
            suspend_cursor: false,
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
                result_max_lines: self.result_max_lines,
                cursor: self.cursor,
                current_request: self.current_request,
                client: self.client,
                suspend_cursor: self.suspend_cursor,
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
            result_max_lines: self.result_max_lines,
            cursor: self.cursor,
            current_request: self.current_request,
            client: self.client,
            suspend_cursor: self.suspend_cursor,
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
        F: for<'a> FnOnce(&'a mut WebwareClient<Registered>) -> BoxFuture<'a, T>,
    {
        let mut client = self.register().await?;
        let result = f(&mut client).await;
        let _ = client.deregister().await?;
        Ok(result)
    }
}

impl<State: Ready> WebwareClient<State> {
    /// Creates a new pagination cursor and makes it available for the next requests (until it is closed)
    pub fn create_cursor(self, max_lines: u32) -> WebwareClient<OpenCursor> {
        let cursor = Cursor::new(max_lines);
        WebwareClient {
            webware_url: self.webware_url,
            vendor_hash: self.vendor_hash,
            app_hash: self.app_hash,
            secret: self.secret,
            revision: self.revision,
            credentials: self.credentials,
            result_max_lines: self.result_max_lines,
            cursor: Some(cursor),
            current_request: self.current_request,
            client: self.client,
            suspend_cursor: self.suspend_cursor,
            state: std::marker::PhantomData::<OpenCursor>,
        }
    }

    /// Generates a set of credentials from the current client.
    pub fn credentials(&self) -> &Credentials {
        self.credentials.as_ref().unwrap()
    }

    /// Sets the maximum amount of results that are returned in a response
    pub fn set_result_max_lines(&mut self, max_lines: u32) {
        self.result_max_lines = max_lines;
    }

    /// Returns a set of headers, that are required on all requests to the WEBSERVICES (except `REGISTER`).
    ///
    /// This will automatically append necessary authentication headers and increase the request ID, if `register()` was successful.
    pub fn get_default_headers(
        &mut self,
        additional_headers: Option<HashMap<&str, &str>>,
    ) -> WWClientResult<HeaderMap> {
        let mut max_lines = self.result_max_lines;

        let mut header_vec = vec![
            ("WWSVC-EXECUTE-MODE", "SYNCHRON".to_string()),
            ("WWSVC-ACCEPT-RESULT-TYPE", "JSON".to_string()),
        ];

        if let Some(credentials) = &self.credentials {
            let app_hash = AppHash::new(self.current_request, &credentials.app_id);
            self.current_request = app_hash.request_id;
            header_vec.append(&mut vec![
                ("WWSVC-REQID", format!("{}", self.current_request)),
                ("WWSVC-TS", app_hash.date_formatted.to_string()),
                ("WWSVC-HASH", format!("{:x}", app_hash)),
            ]);

            if !self.suspend_cursor {
                if let Some(cursor) = &self.cursor {
                    if !cursor.closed() {
                        header_vec
                            .append(&mut vec![("WWSVC-CURSOR", cursor.cursor_id.to_string())]);
                        max_lines = cursor.max_lines;
                    }
                }
            }
        }

        header_vec.push(("WWSVC-ACCEPT-RESULT-MAX-LINES", max_lines.to_string()));

        let mut headers: HashMap<String, String> = header_vec
            .iter()
            .map(|(s1, s2)| (s1.to_string(), s2.to_string()))
            .collect();

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
    pub fn get_bin_headers(
        &mut self,
        additional_headers: Option<HashMap<&str, &str>>,
    ) -> WWClientResult<HeaderMap> {
        let mut headers = self.get_default_headers(additional_headers)?;
        headers.remove("WWSVC-ACCEPT-RESULT-TYPE");
        headers.append("WWSVC-ACCEPT-RESULT-TYPE", HeaderValue::from_str("BIN")?);
        Ok(headers)
    }

    /// Sends a `DEREGISTER` request to the WEBWARE instance, in order to invalidate the service pass.
    pub async fn deregister(mut self) -> WWClientResult<WebwareClient<Unregistered>> {
        let credentials = self.credentials.take();

        if let Some(credentials) = credentials {
            let target_url = self
                .webware_url
                .join("WWSERVICE/")?
                .join("DEREGISTER/")?
                .join(&format!("{}/", &credentials.service_pass))?;
            let headers = self.get_default_headers(None)?;
            let _ = self.client.get(target_url).headers(headers).send().await;
        }

        Ok(WebwareClient {
            webware_url: self.webware_url,
            vendor_hash: self.vendor_hash,
            app_hash: self.app_hash,
            secret: self.secret,
            revision: self.revision,
            credentials: None,
            result_max_lines: self.result_max_lines,
            cursor: self.cursor,
            current_request: self.current_request,
            client: self.client,
            suspend_cursor: self.suspend_cursor,
            state: std::marker::PhantomData::<Unregistered>,
        })
    }

    /// Performs a request to the WEBSERVICES and returns a JSON value.
    pub async fn request(
        &mut self,
        method: reqwest::Method,
        function: &str,
        version: u32,
        parameters: HashMap<&str, &str>,
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
        &mut self,
        method: reqwest::Method,
        function: &str,
        version: u32,
        parameters: HashMap<&str, &str>,
        additional_headers: Option<HashMap<&str, &str>>,
    ) -> WWClientResult<Response> {
        if self.credentials.is_none() {
            return Err(WWSVCError::NotAuthenticated);
        }

        let target_url = self.webware_url.join("EXECJSON")?;
        let headers = self.get_default_headers(additional_headers)?;
        let mut param_vec: Vec<HashMap<String, String>> = Vec::new();
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

        for (p_key, p_value) in parameters {
            let mut map: HashMap<String, String> = HashMap::new();
            map.insert("PNAME".to_string(), p_key.to_string());
            map.insert("PCONTENT".to_string(), p_value.to_string());
            param_vec.push(map);
        }
        let body = json!({
            "WWSVC_FUNCTION": {
                "FUNCTIONNAME": function,
                "PARAMETER": param_vec,
                "REVISION": version
            },
            "WWSVC_PASSINFO": {
                "SERVICEPASS": self.credentials.as_ref().unwrap().service_pass,
                "APPHASH": app_hash,
                "TIMESTAMP": timestamp,
                "REQUESTID": self.current_request,
                "EXECUTE_MODE": "SYNCHRON"
            }
        });
        let response = self
            .client
            .request(method, target_url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        if !self.suspend_cursor {
            if let Some(cursor) = &mut self.cursor {
                if !cursor.closed() && response.headers().contains_key("WWSVC-CURSOR") {
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
        &mut self,
        method: reqwest::Method,
        function: &str,
        version: u32,
        parameters: HashMap<&str, &str>,
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

impl WebwareClient<OpenCursor> {
    /// Suspends the cursor, so that it is not used for the next request
    pub fn suspend_cursor(&mut self) {
        self.suspend_cursor = true;
    }

    /// Resumes the cursor, so that it is used for the next request
    pub fn resume_cursor(&mut self) {
        self.suspend_cursor = false;
    }

    /// Returns whether the current cursor is closed.
    ///
    /// Returns None, if no cursor is available.
    pub fn cursor_closed(&self) -> bool {
        self.cursor.as_ref().unwrap().closed()
    }
}
