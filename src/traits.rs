use std::collections::HashMap;

use crate::WWClientResult;

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
        client: &mut crate::client::WebwareClient<crate::Registered>,
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

    #[cfg(feature = "stream")]
    /// Requests a stream of this data from the server.
    async fn get_stream<'a>(
        client: crate::client::WebwareClient<crate::OpenCursor>,
        mut parameters: HashMap<&'a str, &'a str>,
    ) -> (
        crate::client::WebwareClient<crate::Registered>,
        Box<dyn futures_core::Stream<Item = WWClientResult<Self>> + 'a>,
    )
    where
        Self: Sized + serde::de::DeserializeOwned + 'a,
    {
        parameters.insert("FELDER", Self::FIELDS);
        let (client, data) = client
            .request_generic_stream(
                Self::METHOD,
                Self::FUNCTION,
                Self::VERSION,
                parameters,
                None,
            )
            .await;
        (client, Box::new(data))
    }
}
