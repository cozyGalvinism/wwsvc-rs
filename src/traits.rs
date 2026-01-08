use std::sync::Arc;

use serde::de::DeserializeOwned;

use crate::cursor_response::HasComResult;
use crate::{CursoredRequests, CursoredResponse, HasList, Registered};
use crate::{Ready, WWClientResult, params::Parameters};

/// Trait for the WWSVCGetData derive macro.
#[wwsvc_rs::async_trait]
pub trait WWSVCGetData: Sized + Clone + DeserializeOwned {
    /// The function name of the WWSVC request.
    const FUNCTION: &'static str;
    /// The version of the function.
    const VERSION: u32 = 1;
    /// The function method of the WWSVC request.
    const METHOD: reqwest::Method = reqwest::Method::PUT;
    /// The fields of the struct.
    const FIELDS: &'static str = "";

    /// The response type of the WWSVC request.
    type Response: DeserializeOwned + HasList<Self> + HasComResult;

    /// The container type of the WWSVC request.
    type Container: DeserializeOwned;

    /// Requests this data from the server.
    async fn get(
        client: &crate::client::WebwareClient<impl Ready + Send>,
        mut parameters: Parameters,
    ) -> WWClientResult<Self::Response> {
        parameters = parameters.param("FELDER", Self::FIELDS);
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

    /// Requests this data from the server using a cursor.
    async fn get_cursored(
        client: Arc<crate::client::WebwareClient<Registered>>,
        mut parameters: Parameters,
        max_lines: u32
    ) -> WWClientResult<CursoredResponse<Self, Self::Response>> {
        parameters = parameters.param("FELDER", Self::FIELDS);
        client.cursored_request(Self::METHOD, Self::FUNCTION, Self::VERSION, parameters, max_lines).await
    }
}
