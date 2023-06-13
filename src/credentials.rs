use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
/// Credentials for the client.
pub struct Credentials {
    /// The service pass for the client.
    pub service_pass: String,
    /// The app id for the client.
    pub app_id: String,
}

impl Credentials {
    /// Creates a new `Credentials` struct.
    pub fn new(service_pass: &str, app_id: &str) -> Credentials {
        Credentials {
            service_pass: service_pass.to_string(),
            app_id: app_id.to_string(),
        }
    }
}
