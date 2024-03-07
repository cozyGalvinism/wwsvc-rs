use wwsvc_rs::{collection, WWSVCGetData};

#[derive(WWSVCGetData, Debug, serde::Deserialize, Clone)]
#[wwsvc(function = "ARTIKEL")]
pub struct ArticleData {
    #[serde(rename = "ART_1_25")]
    pub article_number: String,
}

#[tokio::test]
async fn test_articles() {
    dotenv::from_filename("tests/.env").ok();

    let client = wwsvc_rs::WebwareClient::builder()
        .webware_url(std::env::var("WEBWARE_URL").unwrap().as_str())
        .vendor_hash(std::env::var("VENDOR_HASH").unwrap().as_str())
        .app_hash(std::env::var("APP_HASH").unwrap().as_str())
        .secret(std::env::var("APP_SECRET").unwrap().as_str())
        .revision(std::env::var("REVISION").unwrap().parse().unwrap())
        .allow_insecure(true)
        .build();

    let mut registered_client = client.register().await.unwrap();

    let articles = ArticleData::get(
        &mut registered_client,
        collection! {
            "ARTNR" => std::env::var("TEST_ARTNR").unwrap().as_str(),
        },
    )
    .await
    .unwrap();

    assert!(articles.container.list.is_some());
    let list = articles.container.list.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].article_number,
        std::env::var("TEST_ARTNR").unwrap().as_str()
    );

    registered_client.deregister().await.unwrap();
}
