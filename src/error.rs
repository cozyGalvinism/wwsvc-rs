/// Error type for the wwsvc-rs crate.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum WWSVCError {
    /// The client is not authenticated.
    #[error("The client is not authenticated.")]
    #[diagnostic(code(wwsvc_rs::error::WWSVCError::NotAuthenticated))]
    NotAuthenticated,

    /// Missing credentials.
    #[error("Missing credentials.")]
    #[diagnostic(code(wwsvc_rs::error::WWSVCError::MissingCredentials))]
    MissingCredentials,

    /// The request to the server has failed.
    #[error(transparent)]
    #[diagnostic(code(wwsvc_rs::error::WWSVCError::ReqwestError))]
    ReqwestError(#[from] reqwest::Error),

    /// An invalid header value has been provided.
    #[error(transparent)]
    #[diagnostic(code(wwsvc_rs::error::WWSVCError::InvalidHeaderValue))]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    /// A generic HTTP error has occurred.
    #[error(transparent)]
    #[diagnostic(code(wwsvc_rs::error::WWSVCError::HttpError))]
    HttpError(#[from] http::Error),

    /// Header value contains non-ASCII characters.
    #[error(transparent)]
    #[diagnostic(code(wwsvc_rs::error::WWSVCError::HeaderValueToStrError))]
    HeaderValueToStrError(#[from] http::header::ToStrError),

    /// Url parsing error.
    #[error(transparent)]
    #[diagnostic(code(wwsvc_rs::error::WWSVCError::UrlParseError))]
    UrlParseError(#[from] url::ParseError),
}
