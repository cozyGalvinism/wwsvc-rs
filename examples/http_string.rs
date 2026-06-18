use reqwest::Method;
use wwsvc_rs::{requests::RequestToHttpString, Credentials, WebwareClient};

#[tokio::main]
async fn main() {
    let vendor_hash = std::env::var("WW_VENDOR_HASH").expect("WW_VENDOR_HASH not set");
    let app_hash = std::env::var("WW_APP_HASH").expect("WW_APP_HASH not set");
    let revision = std::env::var("WW_REVISION")
        .expect("WW_REVISION not set")
        .parse()
        .unwrap();
    let secret = std::env::var("WW_SECRET").expect("WW_SECRET not set");
    let webware_url = std::env::var("WW_WEBWARE_URL").expect("WW_WEBWARE_URL not set");
    let service_pass = std::env::var("WW_SERVICE_PASS").expect("WW_SERVICE_PASS not set");
    let app_id = std::env::var("WW_APP_ID").expect("WW_APP_ID not set");

    let client = WebwareClient::builder()
        .webware_url(&webware_url)
        .vendor_hash(&vendor_hash)
        .app_hash(&app_hash)
        .secret(&secret)
        .revision(revision)
        .credentials(Credentials::new(&service_pass, &app_id))
        .build();

    let registered_client = client.register().await.expect("failed to register");

    let request = registered_client
        .prepare_request(
            Method::PUT,
            "ARTIKEL.GET",
            1,
            wwsvc_rs::Parameters::new().param("FELDER", "ART_1_25"),
            None,
        )
        .await
        .expect("invalid request");

    println!("{}", request.to_http_string().unwrap());
}
