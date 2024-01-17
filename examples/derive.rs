use wwsvc_rs::{collection, WWSVCGetData};

#[derive(WWSVCGetData, Debug, serde::Deserialize, Clone)]
#[wwsvc(function = "ARTIKEL")]
pub struct ArticleData {
    #[serde(rename = "ART_1_25")]
    pub article_number: String,
}

#[tokio::main]
async fn main() {
    // Construct a new client.
    let client = wwsvc_rs::WebwareClient::builder()
        .webware_url(std::env::var("WEBWARE_URL").unwrap().as_str())
        .vendor_hash(std::env::var("VENDOR_HASH").unwrap().as_str())
        .app_hash(std::env::var("APP_HASH").unwrap().as_str())
        .secret(std::env::var("APP_SECRET").unwrap().as_str())
        .revision(std::env::var("REVISION").unwrap().parse().unwrap())
        // Allow insecure connections. Remove this in for live applications.
        .allow_insecure(true)
        .build();

    // Register the client.
    let mut registered_client = client.register().await.unwrap();

    // Retrieve the article data.
    let articles = ArticleData::get(
        &mut registered_client,
        collection! {
            "ARTNR" => "Artikel19Prozent",
        },
    )
    .await
    .unwrap();

    let list = articles.container.list.unwrap();
    for article in list {
        println!("{:?}", article);
    }

    // Deregister the client.
    registered_client.deregister().await.unwrap();
}