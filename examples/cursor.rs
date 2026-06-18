use reqwest::Method;
use serde::Deserialize;
use std::sync::Arc;
use wwsvc_rs::{Credentials, CursoredRequests, Parameters, WebwareClient, generate_get_response};

// Generate the response type for ARTIKEL.GET
generate_get_response!(ArtikelGetResponse, "ARTIKELLISTE", ArtikelListe, "ARTIKEL");

#[derive(Deserialize, Debug, Clone)]
pub struct ArticleData {
    #[serde(rename = "ART_1_25")]
    pub article_number: String,
}

#[tokio::main]
async fn main() {
    dotenvy::from_filename("tests/.env").ok();

    let vendor_hash = std::env::var("VENDOR_HASH").expect("VENDOR_HASH not set");
    let app_hash = std::env::var("APP_HASH").expect("APP_HASH not set");
    let revision = std::env::var("REVISION")
        .expect("REVISION not set")
        .parse()
        .unwrap();
    let secret = std::env::var("APP_SECRET").expect("APP_SECRET not set");
    let webware_url = std::env::var("WEBWARE_URL").expect("WEBWARE_URL not set");
    let credentials = Credentials::new(std::env::var("SERVICE_PASS").expect("SERVICE_PASS not set").as_str(), std::env::var("APP_ID").expect("APP_ID not set").as_str());

    let client = WebwareClient::builder()
        .webware_url(&webware_url)
        .vendor_hash(&vendor_hash)
        .app_hash(&app_hash)
        .secret(&secret)
        .revision(revision)
        .credentials(credentials)
        .build();

    let registered_client = client.register().await.expect("failed to register");
    let client_arc = Arc::new(registered_client);

    // Example 1: Manual iteration with specific response type
    println!("=== Manual iteration (100 items per page) ===");
    let mut cursor = client_arc
        .cursored_request::<ArticleData, ArtikelGetResponse<ArticleData>>(
            Method::PUT,
            "ARTIKEL.GET",
            1,
            Parameters::new().param("FELDER", "ART_1_25"),
            100, // Page size: 100 items per page
        )
        .await
        .expect("failed to create cursor");

    let mut page_count = 0;
    while let Some(batch) = cursor.next().await.expect("failed to fetch page") {
        page_count += 1;
        println!("Page {}: {} items", page_count, batch.len());
        for item in batch {
            println!("  Article: {}", item.article_number);
        }
        
        // Stop after 3 pages for demo purposes
        if page_count >= 3 {
            break;
        }
    }

    // Example 2: Collect all items
    println!("\n=== Collect all items (50 items per page) ===");
    let mut cursor = client_arc
        .cursored_request::<ArticleData, ArtikelGetResponse<ArticleData>>(
            Method::PUT,
            "ARTIKEL.GET",
            1,
            Parameters::new().param("FELDER", "ART_1_25"),
            50, // Page size: 50 items per page
        )
        .await
        .expect("failed to create cursor");

    let all_items = cursor.collect_all().await.expect("failed to collect all");
    println!("Total items collected: {}", all_items.len());
}
