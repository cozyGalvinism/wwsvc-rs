//! Internal protocol constants (header names and magic string values).

/// Request ID header.
pub(crate) const HEADER_REQUEST_ID: &str = "WWSVC-REQID";
/// Timestamp header.
pub(crate) const HEADER_TIMESTAMP: &str = "WWSVC-TS";
/// Authentication hash header.
pub(crate) const HEADER_HASH: &str = "WWSVC-HASH";
/// Pagination cursor header.
pub(crate) const HEADER_CURSOR: &str = "WWSVC-CURSOR";
/// Execute-mode header.
pub(crate) const HEADER_EXECUTE_MODE: &str = "WWSVC-EXECUTE-MODE";
/// Accepted result type header.
pub(crate) const HEADER_RESULT_TYPE: &str = "WWSVC-ACCEPT-RESULT-TYPE";
/// Maximum result lines header.
pub(crate) const HEADER_RESULT_MAX_LINES: &str = "WWSVC-ACCEPT-RESULT-MAX-LINES";

/// Synchronous execute mode.
pub(crate) const EXECUTE_MODE_SYNCHRON: &str = "SYNCHRON";
/// JSON result type.
pub(crate) const RESULT_TYPE_JSON: &str = "JSON";
/// Binary result type.
pub(crate) const RESULT_TYPE_BIN: &str = "BIN";

/// Cursor id sent to request a fresh cursor.
pub(crate) const CURSOR_CREATE: &str = "CREATE";
/// Cursor id returned once the server has no more results.
pub(crate) const CURSOR_CLOSED: &str = "CLOSED";
