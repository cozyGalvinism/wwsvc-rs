use std::collections::HashMap;
use std::fmt::Write;

use serde::{Deserialize, Serialize};

use crate::WWSVCError;

/// Trait for converting a `reqwest::Request` to a HTTP string.
pub trait RequestToHttpString {
    /// Converts the `reqwest::Request` to a HTTP string.
    fn to_http_string(&self) -> Result<String, WWSVCError>;
}

impl RequestToHttpString for reqwest::Request {
    fn to_http_string(&self) -> Result<String, WWSVCError> {
        let mut result = String::new();

        writeln!(
            result,
            "{} {}{} HTTP/1.1",
            self.method(),
            self.url().path(),
            self.url().query().unwrap_or("")
        )?;

        if let Some(host) = self.url().host_str() {
            if let Some(port) = self.url().port() {
                writeln!(result, "host: {}:{}", host, port)?;
            } else {
                writeln!(result, "host: {}", host)?;
            }
        }

        for (name, value) in self.headers() {
            writeln!(result, "{}: {}", name, value.to_str()?)?;
        }

        writeln!(result)?;

        if let Some(body) = self.body() {
            if let Some(bytes) = body.as_bytes() {
                result.push_str(&String::from_utf8_lossy(bytes));
            } else {
                result.push_str("[streaming body - cannot display]");
            }
        }

        Ok(result)
    }
}

/// The request body for the `EXECJSON` endpoint.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExecJsonRequest {
    /// The service function to be executed.
    #[serde(rename = "WWSVC_FUNCTION")]
    pub function: ServiceFunction,
    /// The authentication info for this request.
    #[serde(rename = "WWSVC_PASSINFO")]
    pub pass_info: ServicePassInfo,
}

impl ExecJsonRequest {
    /// Creates a new `ExecJsonRequest` to be passed as a request body to the `EXECJSON` endpoint.
    pub fn new(
        function_name: &str,
        parameters: Vec<ServiceFunctionParameter>,
        version: u32,
        service_pass: &str,
        app_hash: &str,
        timestamp: &str,
        request_id: u32,
    ) -> Self {
        Self {
            function: ServiceFunction {
                function_name: function_name.to_string(),
                parameters,
                revision: version,
            },
            pass_info: ServicePassInfo {
                service_pass: service_pass.to_string(),
                app_hash: app_hash.to_string(),
                timestamp: timestamp.to_string(),
                request_id,
                execute_mode: "SYNCHRON".to_string(),
            },
        }
    }
}

/// The function to be executed.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServiceFunction {
    /// The name of the function.
    #[serde(rename = "FUNCTIONNAME")]
    pub function_name: String,
    /// The parameters of the function.
    #[serde(rename = "PARAMETER")]
    pub parameters: Vec<ServiceFunctionParameter>,
    /// The revision of the function.
    #[serde(rename = "REVISION")]
    pub revision: u32,
}

/// The parameters of the function.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServiceFunctionParameter {
    /// The name of the parameter.
    #[serde(rename = "PNAME")]
    pub name: String,
    /// The value of the parameter.
    #[serde(rename = "PCONTENT")]
    pub content: String,
}

/// Trait for converting a type to a vector of `ServiceFunctionParameter`.
pub trait ToServiceFunctionParameters {
    /// Converts the type to a vector of `ServiceFunctionParameter`.
    fn to_service_function_parameters(&self) -> Vec<ServiceFunctionParameter>;
}

impl ToServiceFunctionParameters for HashMap<String, String> {
    /// Converts the `HashMap` to a vector of `ServiceFunctionParameter`.
    fn to_service_function_parameters(&self) -> Vec<ServiceFunctionParameter> {
        self.iter()
            .map(|(name, content)| ServiceFunctionParameter {
                name: name.clone(),
                content: content.clone(),
            })
            .collect()
    }
}

impl ToServiceFunctionParameters for HashMap<&str, &str> {
    /// Converts the `HashMap` to a vector of `ServiceFunctionParameter`.
    fn to_service_function_parameters(&self) -> Vec<ServiceFunctionParameter> {
        self.iter()
            .map(|(name, content)| ServiceFunctionParameter {
                name: name.to_string(),
                content: content.to_string(),
            })
            .collect()
    }
}

/// The authentication info for a request.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServicePassInfo {
    /// The service pass.
    #[serde(rename = "SERVICEPASS")]
    pub service_pass: String,
    /// The application hash.
    #[serde(rename = "APPHASH")]
    pub app_hash: String,
    /// The timestamp of the request.
    #[serde(rename = "TIMESTAMP")]
    pub timestamp: String,
    /// The request ID.
    #[serde(rename = "REQUESTID")]
    pub request_id: u32,
    /// The execute mode.
    #[serde(rename = "EXECUTE_MODE")]
    pub execute_mode: String,
}
