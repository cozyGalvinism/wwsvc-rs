use reqwest::Response;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use std::convert::{TryInto};
use std::collections::{HashMap};
use std::future::Future;

use crate::Cursor;
use crate::AppHash;

/// A builder for constructing a WEBWARE client
pub struct WebwareClientBuilder {
    /// Host of the WEBWARE server
    host: String,
    /// Port of the WEBWARE server
    port: u16,
    /// Path to the WEBSERVICES endpoint, relative to the full URL
    webservice_path: String,
    /// Vendor hash of the application
    vendor_hash: String,
    /// Application hash of the application
    app_hash: String,
    /// Application secret, assigned by the WEBWARE instance
    secret: String,
    /// Revision of the application
    revision: u32,
    /// Allow unsafe certificates
    /// (only use this if you know what you are doing)
    /// (default: false)
    allow_unsafe_certs: bool,
    /// The timeout for the request in seconds
    /// (default: 60)
    timeout: u64,
    /// Maximum amount of objects that are returned in a request
    result_max_lines: u32,
}

impl Default for WebwareClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WebwareClientBuilder {
    /// Creates a new instance of the builder
    pub fn new() -> Self {
        WebwareClientBuilder {
            host: String::new(),
            port: 443,
            webservice_path: "/WWSVC/".to_string(),
            vendor_hash: String::new(),
            app_hash: String::new(),
            secret: String::new(),
            revision: 0,
            allow_unsafe_certs: false,
            timeout: 60,
            result_max_lines: 1000,
        }
    }

    /// Sets the host of the WEBWARE server
    /// 
    /// Don't include the protocol (http:// or https://)!
    /// 
    /// (default: "")
    pub fn host(&mut self, host: &str) -> &mut Self {
        self.host = host.to_string();
        self
    }

    /// Sets the port of the WEBWARE server
    /// 
    /// (default: 443)
    pub fn port(&mut self, port: u16) -> &mut Self {
        self.port = port;
        self
    }


    /// Sets the path to the WEBSERVICES endpoint, relative to the full URL
    /// 
    /// (default: "/WWSVC/")
    pub fn webservice_path(&mut self, path: &str) -> &mut Self {
        self.webservice_path = path.to_string();
        self
    }

    /// Sets the vendor hash of the application
    /// 
    /// (default: "")
    pub fn vendor_hash(&mut self, hash: &str) -> &mut Self {
        self.vendor_hash = hash.to_string();
        self
    }

    /// Sets the application hash of the application
    /// 
    /// (default: "")
    pub fn app_hash(&mut self, hash: &str) -> &mut Self {
        self.app_hash = hash.to_string();
        self
    }

    /// Sets the application secret, assigned by the WEBWARE instance
    /// 
    /// (default: "")
    pub fn secret(&mut self, secret: &str) -> &mut Self {
        self.secret = secret.to_string();
        self
    }

    /// Sets the revision of the application
    /// 
    /// (default: 0)
    pub fn revision(&mut self, revision: u32) -> &mut Self {
        self.revision = revision;
        self
    }

    /// Sets whether to allow unsafe certificates
    /// 
    /// (default: false)
    pub fn allow_unsafe_certs(&mut self, allow: bool) -> &mut Self {
        self.allow_unsafe_certs = allow;
        self
    }

    /// Sets the timeout for the request in seconds
    /// 
    /// (default: 60)
    pub fn timeout(&mut self, timeout: u64) -> &mut Self {
        self.timeout = timeout;
        self
    }

    /// Sets the maximum amount of objects that are returned in a response
    /// 
    /// (default: 1000)
    pub fn result_max_lines(&mut self, lines: u32) -> &mut Self {
        self.result_max_lines = lines;
        self
    }

    /// Builds the client
    pub fn build(&self) -> WebwareClient {
        WebwareClient {
            webware_url: format!("https://{}:{}{}", self.host, self.port, self.webservice_path),
            vendor_hash: self.vendor_hash.clone(),
            app_hash: self.app_hash.clone(),
            secret: self.secret.clone(),
            revision: self.revision,
            result_max_lines: self.result_max_lines,
            app_id: None,
            current_request: 0,
            service_pass: None,
            cursor: None,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(self.timeout))
                .danger_accept_invalid_certs(self.allow_unsafe_certs)
                .build()
                .unwrap(),
        }
    }
}

/// The web client to consume SoftENGINE's WEBSERVICES
pub struct WebwareClient {
    /// Full URL to the WEBWARE instance
    webware_url: String,
    /// Vendor hash of the application
    vendor_hash: String,
    /// Application hash of the application
    app_hash: String,
    /// Application secret, assigned by the WEBWARE instance
    secret: String,
    /// Revision of the application
    revision: u32,
    /// Application ID for the service pass, only populated after `register()` is ran
    app_id: Option<String>,
    /// Current request ID
    current_request: u32,
    /// Maximum amount of objects that are returned in a request
    result_max_lines: u32,
    /// Service pass of the client, only populated after `register()` is ran
    service_pass: Option<String>,
    /// Internal reqwest client
    client: reqwest::Client,
    /// Request cursor for pagination,
    cursor: Option<Cursor>,
}

impl WebwareClient {
    /// Creates a new pagination cursor and makes it available for the next requests (until it is closed)
    pub fn create_cursor(&mut self, max_lines: u32) {
        self.cursor = Some(Cursor::new(max_lines));
    }

    /// Returns whether the current cursor is closed.
    /// 
    /// Returns true if there is no cursor.
    pub fn cursor_closed(&self) -> bool {
        if let Some(c) = self.cursor.as_ref() {
            c.closed()
        } else {
            true
        }
    }

    /// Sets the maximum amount of results that are returned in a response
    pub fn set_result_max_lines(&mut self, max_lines: u32) {
        self.result_max_lines = max_lines;
    }

    /// Returns a set of headers, that are required on all requests to the WEBSERVICES (except `REGISTER`).
    ///
    /// This will automatically append necessary authentication headers and increase the request ID, if `register()` was successful.
    pub fn get_default_headers(&mut self, additional_headers: Option<HashMap<&str, &str>>) -> HeaderMap {
        let mut max_lines = self.result_max_lines;

        let mut header_vec = vec![
            ("WWSVC-EXECUTE-MODE", "SYNCHRON".to_string()),
            ("WWSVC-ACCEPT-RESULT-TYPE", "JSON".to_string())
        ];
        
        if self.app_id.is_some() {
            let app_id = self.app_id.as_deref().expect("msg");
            let app_hash = AppHash::new(self.current_request, app_id.to_string());
            self.current_request = app_hash.request_id;
            header_vec.append(&mut vec![
                ("WWSVC-REQID", format!("{}", self.current_request)),
                ("WWSVC-TS",  app_hash.date_formatted.to_string()),
                ("WWSVC-HASH", format!("{:x}", app_hash))
            ]);

            if let Some(cursor) = &self.cursor {
                if !cursor.closed() {
                    header_vec.append(&mut vec![
                        ("WWSVC-CURSOR", cursor.cursor_id.to_string())
                    ]);
                    max_lines = cursor.max_lines;
                }
            }
        }

        header_vec.push(("WWSVC-ACCEPT-RESULT-MAX-LINES", format!("{}", max_lines)));

        let mut headers: HashMap<String, String> = header_vec.iter()
            .map(|(s1, s2)|(s1.to_string(), s2.to_string()))
            .collect();
        
        if let Some(additional_headers) = additional_headers {
            for (key, value) in additional_headers {
                headers.insert(key.to_string(), value.to_string());
            }
        }

        (&headers).try_into().expect("invalid headers")
    }

    /// Returns the same set of headers, that `get_default_headers()` returns, except the result type header is set to `BIN` instead.
    pub fn get_bin_headers(&mut self, additional_headers: Option<HashMap<&str, &str>>) -> HeaderMap {
        let mut headers = self.get_default_headers(additional_headers);
        headers.remove("WWSVC-ACCEPT-RESULT-TYPE");
        headers.append("WWSVC-ACCEPT-RESULT-TYPE", HeaderValue::from_str("BIN").expect("valid header"));
        headers
    }

    /// Builds a valid WEBSERVICES URL from URL parts
    pub fn build_url(&self, parts: Vec<String>) -> String {
        let append = parts.join("/");
        return format!("{}{}", self.webware_url, append);
    }

    /// Sends a `REGISTER` request to the WEBWARE instance and returns whether the request succeeded or not.
    ///
    /// If the result is not `true`, the client has no valid service pass and cannot perform requests!
    pub async fn register(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let target_url = self.build_url(vec!["WWSERVICE".to_string(), "REGISTER".to_string(), self.vendor_hash.clone(), self.app_hash.clone(), self.secret.clone(), self.revision.clone().to_string()]);
        let response = self.client.get(target_url).send().await?;
        let response_obj = response.json::<HashMap<String, serde_json::Value>>().await?;

        if !response_obj.contains_key("SERVICEPASS") {
            return Ok(false);
        }

        let service_pass = response_obj["SERVICEPASS"].as_object().unwrap();
        self.service_pass = Some(service_pass["PASSID"].as_str().unwrap().to_string());
        self.app_id = Some(service_pass["APPID"].as_str().unwrap().to_string());

        Ok(true)
    }

    /// Sends a `DEREGISTER` request to the WEBWARE instance, in order to invalidate the service pass.
    ///
    /// If the client was not authenticated using `register()` before, it will instead just return true;
    pub async fn deregister(&mut self) -> bool {
        if self.service_pass.is_none() {
            return true;
        }
        let target_url = self.build_url(vec!["WWSERVICE".to_string(), "DEREGISTER".to_string(), self.service_pass.clone().unwrap()]);
        let headers = self.get_default_headers(None);
        let _ = self.client.get(target_url).headers(headers).send().await;
        self.service_pass = None;
        self.app_id = None;
        true
    }

    /// Performs a request to the WEBSERVICES and returns a JSON value.
    pub async fn request(&mut self, method: reqwest::Method, function: &str, version: u32, parameters: HashMap<&str, &str>, additional_headers: Option<HashMap<&str, &str>>) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        return self.request_generic::<serde_json::Value>(method, function, version, parameters, additional_headers).await;
    }

    /// Performs a request to the WEBSERVICES and returns a response object.
    pub async fn request_as_response(&mut self, method: reqwest::Method, function: &str, version: u32, parameters: HashMap<&str, &str>, additional_headers: Option<HashMap<&str, &str>>) -> Result<Response, Box<dyn std::error::Error>> {
        let target_url = self.build_url(vec!["EXECJSON".to_string()]);
        let headers = self.get_default_headers(additional_headers);
        let mut param_vec: Vec<HashMap<String, String>> = Vec::new();
        let app_hash_header = headers.get("WWSVC-HASH");
        let timestamp_header = headers.get("WWSVC-TS");
        let app_hash: String = app_hash_header.unwrap_or(&HeaderValue::from_str("").unwrap()).to_str().expect("msg").to_string();
        let timestamp: String = timestamp_header.unwrap_or(&HeaderValue::from_str("").unwrap()).to_str().expect("msg").to_string();

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
                "SERVICEPASS": self.service_pass.as_deref().unwrap_or(""),
                "APPHASH": app_hash,
                "TIMESTAMP": timestamp,
                "REQUESTID": self.current_request,
                "EXECUTE_MODE": "SYNCHRON"
            }
        });
        let response = self.client.request(method, target_url)
            .headers(headers)
            .json(&body)
            .send().await?;
        
        if let Some(cursor) = &mut self.cursor {
            if !cursor.closed() && response.headers().contains_key("WWSVC-CURSOR") {
                cursor.set_cursor_id(response.headers().get("WWSVC-CURSOR").unwrap().to_str().unwrap().to_string());
            }
        }
        
        Ok(response)
    }

    /// Performs a request to the WEBSERVICES and deserializes the response to the type `T`.
    ///
    /// **NOTE:** Due to the nature of the WEBSERVICES, deserialization might fail due to structural issues. In that case, use `request()` instead.
    pub async fn request_generic<T>(&mut self, method: reqwest::Method, function: &str, version: u32, parameters: HashMap<&str, &str>, additional_headers: Option<HashMap<&str, &str>>) -> Result<T, Box<dyn std::error::Error>> 
    where
        T:DeserializeOwned
    {
        let response = self.request_as_response(method, function, version, parameters, additional_headers).await?;
        let response_obj = response.json::<T>().await?;
        Ok(response_obj)
    }
}