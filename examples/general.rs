use reqwest::Method;
use serde::Deserialize;
use wwsvc_rs::{WebwareClient, Registered, collection, generate_get_response};

async fn get_json_value(client: &mut WebwareClient<Registered>) {
    let json_value = client
        .request(Method::PUT, "ARTIKEL.GET", 1, collection! {
            "FELDER" => "ART_1_25",
        }, None)
        .await
        .unwrap();

    println!("{:#?}", json_value);
}

#[derive(Deserialize, Debug, Clone)]
pub struct ArticleData {
    #[serde(rename = "ART_1_25")]
    pub article_number: String,
}

generate_get_response!(Articles, "ARTIKELLISTE", ArticleList, "ARTIKEL");

async fn get_deserialized_value(client: &mut WebwareClient<Registered>) {
    let articles = client
        .request_generic::<Articles<ArticleData>>(Method::PUT, "ARTIKEL.GET", 1, collection! {
            "FELDER" => "ART_1_25",
        }, None)
        .await
        .unwrap();

    println!("{:#?}", articles.container.list);
}

#[tokio::main]
async fn main() {
    let vendor_hash = std::env::var("WW_VENDOR_HASH").expect("WW_VENDOR_HASH not set");
    let app_hash = std::env::var("WW_APP_HASH").expect("WW_APP_HASH not set");
    let revision = std::env::var("WW_REVISION").expect("WW_REVISION not set").parse().unwrap();
    let secret = std::env::var("WW_SECRET").expect("WW_SECRET not set");
    let webware_url = std::env::var("WW_WEBWARE_URL").expect("WW_WEBWARE_URL not set");

    let client = WebwareClient::builder()
        .webware_url(&webware_url)
        .vendor_hash(&vendor_hash)
        .app_hash(&app_hash)
        .secret(&secret)
        .revision(revision)
        .build();

    let mut registered_client = client.register().await.expect("failed to register");

    get_json_value(&mut registered_client).await;
    get_deserialized_value(&mut registered_client).await;

    registered_client.deregister().await.expect("failed to deregister");
}
