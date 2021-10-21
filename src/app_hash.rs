use httpdate::fmt_http_date;
use encoding_rs::WINDOWS_1252;
use std::time::{SystemTime};

/// Represents a request hash object, used for securing requests
pub struct AppHash {
    /// The used request ID
    pub request_id: u32,
    /// The resulting hash as String
    pub hash: String,
    /// The current date, formatted as IMF-fixdate
    pub date_formatted: String
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