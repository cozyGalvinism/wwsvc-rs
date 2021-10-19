use reqwest::Response;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use std::convert::{TryInto};
use std::time::{SystemTime};
use std::collections::{HashMap};
use httpdate::fmt_http_date;
use encoding_rs::WINDOWS_1252;

use crate::cursor::Cursor;

/// Represents a request hash object, used for securing requests
pub struct AppHash {
    /// The used request ID
    pub request_id: u32,
    /// The resulting hash as String
    pub hash: String,
    /// The current date, formatted as IMF-fixdate
    pub date_formatted: String
}

/// The web client to consume SoftENGINE's WEBSERVICES
pub struct WebwareClient {
    /// Full URL to the WEBWARE instance
    pub webware_url: String,
    /// Path to the WEBSERVICES endpoint, relative to the full URL
    pub webservice_path: String,
    /// Vendor hash of the application
    pub vendor_hash: String,
    /// Application hash of the application
    pub app_hash: String,
    /// Application secret, assigned by the WEBWARE instance
    pub secret: String,
    /// Revision of the application
    pub revision: u32,
    /// Application ID for the service pass, only populated after `register()` is ran
    pub app_id: Option<String>,
    /// Current request ID
    pub current_request: u32,
    /// Maximum amount of objects that are returned in a request
    pub result_max_lines: u32,
    /// Service pass of the client, only populated after `register()` is ran
    pub service_pass: Option<String>,
    /// Internal reqwest client
    pub client: reqwest::Client,
    /// Request cursor for pagination,
    pub cursor: Option<Cursor>,
}

impl WebwareClient {
    /// Returns a new webservice client, that can be used to consume SoftENGINE's WEBSERVICES
    ///
    /// You can allow access to insecure instances by setting `allow_unsafe_certs` to `true`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(vendor_hash: String, app_hash: String, secret: String, revision: u32, host: String, port: u16, wwsvc_path: String, allow_unsafe_certs: bool) -> Self {
        let ww_url = format!("https://{}:{}{}", host, port, wwsvc_path);
        Self {
            webservice_path: wwsvc_path,
            webware_url: ww_url,
            vendor_hash,
            app_hash,
            secret,
            revision,
            app_id: None,
            current_request: 0,
            result_max_lines: 100000,
            service_pass: None,
            client: reqwest::Client::builder()
                .danger_accept_invalid_certs(allow_unsafe_certs)
                .build()
                .unwrap(),
            cursor: None,
        }
    }

    /// Creates a new pagination cursor and makes it available for the next requests (until it is closed)
    pub fn create_cursor(&mut self, max_lines: u32) {
        self.cursor = Some(Cursor::new(max_lines));
    }

    /// Returns a set of headers, that are required on all requests to the WEBSERVICES (except `REGISTER`).
    ///
    /// This will automatically append necessary authentication headers and increase the request ID, if `register()` was successful.
    pub fn get_default_headers(&mut self) -> HeaderMap {
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

        let headers: HashMap<String, String> = header_vec.iter()
            .map(|(s1, s2)|(s1.to_string(), s2.to_string()))
            .collect();

        (&headers).try_into().expect("invalid headers")
    }

    /// Returns the same set of headers, that `get_default_headers()` returns, except the result type header is set to `BIN` instead.
    pub fn get_bin_headers(&mut self) -> HeaderMap {
        let mut headers = self.get_default_headers();
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
        let headers = self.get_default_headers();
        let _ = self.client.get(target_url).headers(headers).send().await;
        self.service_pass = None;
        self.app_id = None;
        true
    }

    /// Performs a request to the WEBSERVICES and returns a JSON value.
    pub async fn request(&mut self, method: reqwest::Method, function: String, version: u32, parameters: HashMap<String, String>) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        return self.request_generic::<serde_json::Value>(method, function, version, parameters).await;
    }

    /// Performs a request to the WEBSERVICES and returns a response object.
    pub async fn request_as_response(&mut self, method: reqwest::Method, function: String, version: u32, parameters: HashMap<String, String>) -> Result<Response, Box<dyn std::error::Error>> {
        let target_url = self.build_url(vec!["EXECJSON".to_string()]);
        let headers = self.get_default_headers();
        let mut param_vec: Vec<HashMap<String, String>> = Vec::new();
        let app_hash_header = headers.get("WWSVC-HASH");
        let timestamp_header = headers.get("WWSVC-TS");
        let app_hash: String = app_hash_header.unwrap_or(&HeaderValue::from_str("").unwrap()).to_str().expect("msg").to_string();
        let timestamp: String = timestamp_header.unwrap_or(&HeaderValue::from_str("").unwrap()).to_str().expect("msg").to_string();

        for (p_key, p_value) in parameters {
            let mut map: HashMap<String, String> = HashMap::new();
            map.insert("PNAME".to_string(), p_key);
            map.insert("PCONTENT".to_string(), p_value);
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
    pub async fn request_generic<T>(&mut self, method: reqwest::Method, function: String, version: u32, parameters: HashMap<String, String>) -> Result<T, Box<dyn std::error::Error>> 
    where
        T:DeserializeOwned
    {
        let response = self.request_as_response(method, function, version, parameters).await?;
        let response_obj = response.json::<T>().await?;
        Ok(response_obj)
    }
}

impl AppHash {
    /// Returns a new AppHash object from the current request ID and the application secret of a `WebwareClient`.
    ///
    /// Can be formatted as lowercase hexadecimal for ease of use.
    pub fn new(request_id: u32, app_secret: String) -> AppHash {
        let now = fmt_http_date(SystemTime::now());
        let new_request_id = request_id + 1;
        let combined = format!("{}{}", app_secret, now);
        let (cow, _encoding_used, _had_errors) = WINDOWS_1252.encode(&combined[..]);
        let md5_hash = format!("{:x}", md5::compute(cow));
        AppHash {
            request_id: new_request_id,
            hash: md5_hash,
            date_formatted: now
        }
    }
}

impl std::fmt::LowerHex for AppHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(&self.hash)
    }
}