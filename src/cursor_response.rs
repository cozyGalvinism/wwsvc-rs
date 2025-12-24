use std::marker::PhantomData;
use std::sync::Arc;

use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::client::states::Registered;
use crate::params::Parameters;
use crate::{WebwareClient, WWClientResult};

/// Trait for response types that contain a list of items.
/// This allows CursoredResponse to work with any response structure.
pub trait HasList<T> {
    /// Extract the list of items from the response.
    fn into_items(self) -> Option<Vec<T>>;
}

/// A cursor-based response that allows iterating over paginated results.
/// 
/// This struct handles the cursor lifecycle internally, providing a simple
/// interface to fetch pages of results without manually managing cursor state.
/// 
/// The generic parameter R represents the full response type (e.g., ArtikelGetResponse).
pub struct CursoredResponse<T, R>
where
    R: HasList<T>,
{
    client: Arc<WebwareClient<Registered>>,
    method: Method,
    function: String,
    version: u32,
    base_params: Parameters,
    page_size: u32,
    _phantom: PhantomData<(T, R)>,
    finished: bool,
}

impl<T, R> CursoredResponse<T, R>
where
    T: DeserializeOwned + Clone,
    R: DeserializeOwned + HasList<T>,
{
    /// Create a new cursored response.
    pub(crate) fn new(
        client: Arc<WebwareClient<Registered>>,
        method: Method,
        function: String,
        version: u32,
        base_params: Parameters,
        page_size: u32,
    ) -> Self {
        Self {
            client,
            method,
            function,
            version,
            base_params,
            page_size,
            _phantom: PhantomData,
            finished: false,
        }
    }

    /// Fetch the next page of results.
    /// 
    /// Returns None when there are no more pages available.
    pub async fn next(&mut self) -> WWClientResult<Option<Vec<T>>> {
        if self.finished {
            return Ok(None);
        }

        // Create a cursor if this is the first request
        if !self.client.has_cursor().await {
            self.client.create_cursor(self.page_size).await;
        }

        // Make the request
        let response = self
            .client
            .request_generic::<R>(
                self.method.clone(),
                &self.function,
                self.version,
                self.base_params.clone(),
                None,
            )
            .await?;

        // Check if cursor is closed
        if self.client.cursor_closed().await {
            self.finished = true;
            self.client.close_cursor().await;
        }

        // Extract the list using the HasList trait
        let items = response.into_items();
        
        match items {
            Some(ref list) if list.is_empty() => {
                self.finished = true;
                self.client.close_cursor().await;
                Ok(None)
            }
            Some(list) => Ok(Some(list)),
            None => {
                self.finished = true;
                self.client.close_cursor().await;
                Ok(None)
            }
        }
    }

    /// Collect all remaining pages into a single Vec.
    pub async fn collect_all(&mut self) -> WWClientResult<Vec<T>> {
        let mut all_items = Vec::new();
        while let Some(batch) = self.next().await? {
            all_items.extend(batch);
        }
        Ok(all_items)
    }

    /// Check if all pages have been fetched.
    pub fn is_finished(&self) -> bool {
        self.finished
    }
}

/// Extension trait for WebwareClient to provide cursored request methods.
pub trait CursoredRequests {
    /// Make a request that returns a CursoredResponse for paginated results.
    /// 
    /// The generic parameter R represents the full response type.
    /// 
    /// # Arguments
    /// * `page_size` - Number of rows per page (default: 500)
    fn cursored_request<T, R>(
        &self,
        method: Method,
        function: &str,
        version: u32,
        params: Parameters,
        page_size: u32,
    ) -> impl std::future::Future<Output = WWClientResult<CursoredResponse<T, R>>> + Send
    where
        T: DeserializeOwned + Clone,
        R: DeserializeOwned + HasList<T>;
}

impl CursoredRequests for WebwareClient<Registered> {
    fn cursored_request<T, R>(
        &self,
        _method: Method,
        _function: &str,
        _version: u32,
        _params: Parameters,
        _page_size: u32,
    ) -> impl std::future::Future<Output = WWClientResult<CursoredResponse<T, R>>> + Send
    where
        T: DeserializeOwned + Clone,
        R: DeserializeOwned + HasList<T>,
    {
        async move {
            // Cursored requests require an Arc<WebwareClient<Registered>> for shared ownership
            // Please wrap your client in Arc before calling cursored_request
            Err(crate::error::WWSVCError::NotAuthenticated)
        }
    }
}

impl CursoredRequests for Arc<WebwareClient<Registered>> {
    fn cursored_request<T, R>(
        &self,
        method: Method,
        function: &str,
        version: u32,
        params: Parameters,
        page_size: u32,
    ) -> impl std::future::Future<Output = WWClientResult<CursoredResponse<T, R>>> + Send
    where
        T: DeserializeOwned + Clone,
        R: DeserializeOwned + HasList<T>,
    {
        async move {
            Ok(CursoredResponse::new(
                self.clone(),
                method,
                function.to_string(),
                version,
                params,
                page_size,
            ))
        }
    }
}
