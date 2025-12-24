use futures::FutureExt;
use reqwest::Method;
use wwsvc_rs::{generate_get_response, Parameters};

#[derive(Debug, serde::Deserialize, Clone)]
pub struct ArticleData {
    #[serde(rename = "ART_1_25")]
    pub article_number: String,
}

generate_get_response!(ArticleResponse, "ARTIKELLISTE", ArticleContainer, "ARTIKEL");

#[tokio::test]
async fn test_articles() {
    dotenvy::from_filename("tests/.env").ok();

    let client = wwsvc_rs::WebwareClient::builder()
        .webware_url(std::env::var("WEBWARE_URL").unwrap().as_str())
        .vendor_hash(std::env::var("VENDOR_HASH").unwrap().as_str())
        .app_hash(std::env::var("APP_HASH").unwrap().as_str())
        .secret(std::env::var("APP_SECRET").unwrap().as_str())
        .revision(std::env::var("REVISION").unwrap().parse().unwrap())
        .allow_insecure(true)
        .build();

    let articles = client
        .with_registered(|registered_client| {
            async {
                registered_client
                    .request_generic::<ArticleResponse<ArticleData>>(
                        Method::PUT,
                        "ARTIKEL.GET",
                        1,
                        Parameters::new().param("ARTNR", std::env::var("TEST_ARTNR").unwrap().as_str()),
                        None,
                    )
                    .await
            }
            .boxed()
        })
        .await
        .unwrap()
        .unwrap();

    assert!(articles.container.list.is_some());
    let list = articles.container.list.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].article_number,
        std::env::var("TEST_ARTNR").unwrap().as_str()
    );
}
