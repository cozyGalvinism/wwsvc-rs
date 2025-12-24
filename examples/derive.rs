use serde::Deserialize;
use wwsvc_rs::{Parameters, WWSVCGetData, WebwareClient};

#[derive(WWSVCGetData, Debug, Deserialize, Clone)]
#[wwsvc(function = "ARTIKEL")]
pub struct ArticleData {
    #[serde(rename = "ART_1_25")]
    pub article_number: String,
}

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

    let client = WebwareClient::builder()
        .webware_url(&webware_url)
        .vendor_hash(&vendor_hash)
        .app_hash(&app_hash)
        .secret(&secret)
        .revision(revision)
        .build();

    let registered_client = client.register().await.expect("failed to register");

    let articles = ArticleData::get(&registered_client, Parameters::new())
        .await
        .unwrap();

    println!("{:#?}", articles);
}
