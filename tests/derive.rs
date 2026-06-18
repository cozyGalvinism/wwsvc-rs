use std::sync::Arc;

use wwsvc_rs::{Credentials, Parameters, WWSVCGetData};

#[derive(WWSVCGetData, Debug, serde::Deserialize, Clone)]
#[wwsvc(function = "ARTIKEL")]
pub struct ArticleData {
    #[serde(rename = "ART_1_25")]
    pub article_number: String,
}

#[tokio::test]
async fn test_articles() {
    dotenvy::from_filename("tests/.env").ok();

    let _guard = init_tracing_opentelemetry::TracingConfig::testing()
        .init_subscriber()
        .unwrap();

    let client = wwsvc_rs::WebwareClient::builder()
        .webware_url(std::env::var("WEBWARE_URL").unwrap().as_str())
        .vendor_hash(std::env::var("VENDOR_HASH").unwrap().as_str())
        .app_hash(std::env::var("APP_HASH").unwrap().as_str())
        .secret(std::env::var("APP_SECRET").unwrap().as_str())
        .revision(std::env::var("REVISION").unwrap().parse().unwrap())
        .credentials(Credentials::new(std::env::var("SERVICE_PASS").unwrap().as_str(), std::env::var("APP_ID").unwrap().as_str()))
        .allow_insecure(true)
        .build();

    let registered_client = client.register().await.unwrap();

    let response = registered_client.request_as_response(ArticleData::METHOD, "ARTIKEL.GET", 1, Parameters::new().param("ARTNR", std::env::var("TEST_ARTNR").unwrap().as_str()), None).await.unwrap();
    let response_txt = response.text().await.unwrap();
    tracing::debug!("Response: {}", response_txt);
    let articles = ArticleData::get(
        &registered_client,
        Parameters::new().param("ARTNR", std::env::var("TEST_ARTNR").unwrap().as_str()),
    )
    .await
    .unwrap();

    assert!(articles.container.is_some());
    let container = articles.container.unwrap();
    assert!(container.list.is_some());
    let list = container.list.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].article_number,
        std::env::var("TEST_ARTNR").unwrap().as_str()
    );
}

#[tokio::test]
async fn test_articles_cursored() {
    dotenvy::from_filename("tests/.env").ok();

    let _guard = init_tracing_opentelemetry::TracingConfig::testing()
        .init_subscriber()
        .unwrap();

    let client = wwsvc_rs::WebwareClient::builder()
        .webware_url(std::env::var("WEBWARE_URL").unwrap().as_str())
        .vendor_hash(std::env::var("VENDOR_HASH").unwrap().as_str())
        .app_hash(std::env::var("APP_HASH").unwrap().as_str())
        .secret(std::env::var("APP_SECRET").unwrap().as_str())
        .revision(std::env::var("REVISION").unwrap().parse().unwrap())
        .credentials(Credentials::new(std::env::var("SERVICE_PASS").unwrap().as_str(), std::env::var("APP_ID").unwrap().as_str()))
        .allow_insecure(true)
        .build();

    let registered_client = client.register().await.unwrap();
    let cursored = ArticleData::get_cursored(Arc::new(registered_client), Parameters::default(), 10).await;
    assert!(cursored.is_ok());
    let mut cursored = cursored.unwrap();
    let next = cursored.next().await;
    assert!(next.is_ok());
    let next = next.unwrap();
    assert!(next.is_some());
    let next = next.unwrap();
    assert_eq!(next.len(), 10);
}
