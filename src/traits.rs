use std::collections::HashMap;

use crate::{Ready, WWClientResult};

/// Trait for the WWSVCGetData derive macro.
#[cfg(feature = "derive")]
#[wwsvc_rs::async_trait]
pub trait WWSVCGetData {
    /// The function name of the WWSVC request.
    const FUNCTION: &'static str;
    /// The version of the function.
    const VERSION: u32 = 1;
    /// The function method of the WWSVC request.
    const METHOD: reqwest::Method = reqwest::Method::PUT;
    /// The fields of the struct.
    const FIELDS: &'static str = "";

    /// The response type of the WWSVC request.
    type Response: serde::de::DeserializeOwned;

    /// The container type of the WWSVC request.
    type Container: serde::de::DeserializeOwned;

    /// Requests this data from the server.
    async fn get(
        client: &crate::client::WebwareClient<impl Ready + Send>,
        mut parameters: HashMap<&str, &str>,
    ) -> WWClientResult<Self::Response> {
        parameters.insert("FELDER", Self::FIELDS);
        client
            .request_generic(
                Self::METHOD,
                Self::FUNCTION,
                Self::VERSION,
                parameters,
                None,
            )
            .await
    }
}
