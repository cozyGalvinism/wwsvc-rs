use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::client::states::Registered;
use crate::consts::{HEADER_CURSOR, HEADER_RESULT_MAX_LINES};
use crate::cursor::Cursor;
use crate::params::Parameters;
use crate::responses::ComResult;
use crate::{WWClientResult, WebwareClient};

/// Trait for response types that contain a list of items.
/// This allows CursoredResponse to work with any response structure.
pub trait HasList<T> {
    /// Extract the list of items from the response.
    fn into_items(self) -> Option<Vec<T>>;
}

/// Trait for response types that contain a COMRESULT.
pub trait HasComResult {
    /// Returns the COMRESULT of the response.
    fn comresult(&self) -> &ComResult;
}

/// A cursor-based response that allows iterating over paginated results.
///
/// This struct owns its own pagination cursor and handles the cursor lifecycle internally,
/// providing a simple interface to fetch pages of results without manually managing cursor
/// state. Because the cursor lives here (not on the client), you can have multiple
/// independent cursored responses in flight against the same client.
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
    cursor: Cursor,
    _phantom: PhantomData<(T, R)>,
    finished: bool,
}

/// A wrapper that combines a list of items with their associated COMRESULT.
///
/// This struct is useful when you need access to both the data items and the
/// metadata from the COMRESULT (such as status codes, messages, or other
/// response information) in a single structure.
pub struct ItemsWithComResult<T: Clone> {
    /// The list of items returned by the request, if any.
    pub items: Option<Vec<T>>,
    /// The COMRESULT containing metadata about the request.
    pub comresult: ComResult,
}

impl<T: Clone> ItemsWithComResult<T> {
    /// Creates a new ItemsWithComResult with the provided items and COMRESULT.
    ///
    /// # Arguments
    ///
    /// * `items` - A slice of items to include in the response
    /// * `comresult` - The COMRESULT from the server response
    pub fn items(items: &[T], comresult: ComResult) -> Self {
        Self {
            items: Some(items.to_vec()),
            comresult,
        }
    }

    /// Creates a new ItemsWithComResult with no items and the provided COMRESULT.
    ///
    /// # Arguments
    ///
    /// * `comresult` - The COMRESULT from the server response
    pub fn no_items(comresult: ComResult) -> Self {
        Self {
            items: None,
            comresult,
        }
    }
}

impl<T, R> CursoredResponse<T, R>
where
    T: DeserializeOwned + Clone,
    R: DeserializeOwned + HasList<T> + HasComResult,
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
            cursor: Cursor::new(),
            _phantom: PhantomData,
            finished: false,
        }
    }

    /// Fetches the next page, sending this response's own cursor and reading the updated
    /// cursor id back from the response headers.
    async fn fetch(&mut self) -> WWClientResult<Option<ItemsWithComResult<T>>> {
        if self.finished {
            return Ok(None);
        }

        // Send this response's cursor id and page size as per-request headers.
        let page_size = self.page_size.to_string();
        let mut headers = HashMap::new();
        headers.insert(HEADER_CURSOR, self.cursor.cursor_id.as_str());
        headers.insert(HEADER_RESULT_MAX_LINES, page_size.as_str());

        let response = self
            .client
            .request_as_response(
                self.method.clone(),
                &self.function,
                self.version,
                self.base_params.clone(),
                Some(headers),
            )
            .await?;

        // Update our cursor from the response header, if present.
        if let Some(cursor_id) = response
            .headers()
            .get(HEADER_CURSOR)
            .and_then(|v| v.to_str().ok())
        {
            self.cursor.set_cursor_id(cursor_id.to_string());
        }

        let parsed = response.json::<R>().await?;
        let comresult = parsed.comresult().clone();

        if self.cursor.closed() {
            tracing::debug!(comresult = ?comresult, "Cursor closed as indicated by the server");
            self.finished = true;
        }

        match parsed.into_items() {
            Some(list) if list.is_empty() => {
                tracing::warn!(comresult = ?comresult, "Empty list received from server, closing cursor");
                self.finished = true;
                Ok(Some(ItemsWithComResult {
                    items: None,
                    comresult,
                }))
            }
            Some(list) => Ok(Some(ItemsWithComResult {
                items: Some(list),
                comresult,
            })),
            None => {
                tracing::warn!(comresult = ?comresult, "No list received from server, closing cursor");
                self.finished = true;
                Ok(Some(ItemsWithComResult {
                    items: None,
                    comresult,
                }))
            }
        }
    }

    /// Fetch the next page of results together with its COMRESULT.
    ///
    /// Returns None when there are no more pages available.
    pub async fn next_with_comresult(&mut self) -> WWClientResult<Option<ItemsWithComResult<T>>> {
        self.fetch().await
    }

    /// Fetch the next page of results.
    ///
    /// Returns None when there are no more pages available.
    pub async fn next(&mut self) -> WWClientResult<Option<Vec<T>>> {
        match self.fetch().await? {
            Some(ItemsWithComResult {
                items: Some(items), ..
            }) if !items.is_empty() => Ok(Some(items)),
            _ => Ok(None),
        }
    }

    /// Collect all remaining pages into a single Vec.
    pub async fn collect_all(mut self) -> WWClientResult<Vec<T>> {
        let mut all_items = Vec::new();
        while let Some(batch) = self.next().await? {
            all_items.extend(batch);
        }
        Ok(all_items)
    }

    /// Collect all remaining pages into a single Vec.
    ///
    /// This version uses `next_with_comresult` internally for better tracing
    /// and debugging output during the collection process.
    pub async fn collect_all_with_comresult(mut self) -> WWClientResult<Vec<T>> {
        let mut all_items = Vec::new();
        while let Some(batch) = self.next_with_comresult().await? {
            if let Some(items) = batch.items {
                all_items.extend(items);
            }
        }
        Ok(all_items)
    }

    /// Check if all pages have been fetched.
    pub fn is_finished(&self) -> bool {
        self.finished
    }
}

/// Extension trait for WebwareClient to provide cursored request methods.
///
/// This is implemented for `Arc<WebwareClient<Registered>>` only: a [`CursoredResponse`]
/// must own a shared client for its whole lifetime, so the client has to be wrapped in an
/// [`Arc`] first. Calling this on a bare `WebwareClient<Registered>` is a compile error.
pub trait CursoredRequests {
    /// Make a request that returns a CursoredResponse for paginated results.
    ///
    /// The generic parameter R represents the full response type.
    ///
    /// # Arguments
    /// * `page_size` - Number of rows per page (required, no default).
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
        R: DeserializeOwned + HasList<T> + HasComResult;
}

impl CursoredRequests for Arc<WebwareClient<Registered>> {
    async fn cursored_request<T, R>(
        &self,
        method: Method,
        function: &str,
        version: u32,
        params: Parameters,
        page_size: u32,
    ) -> WWClientResult<CursoredResponse<T, R>>
    where
        T: DeserializeOwned + Clone,
        R: DeserializeOwned + HasList<T> + HasComResult,
    {
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
