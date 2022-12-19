use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
/// Credentials for the client.
pub struct Credentials {
    /// The service pass for the client.
    pub service_pass: String,
    /// The app id for the client.
    pub app_id: String,
}

impl Credentials {
    /// Creates a new `Credentials` struct.
    pub fn new(service_pass: String, app_id: String) -> Credentials {
        Credentials {
            service_pass,
            app_id,
        }
    }
}