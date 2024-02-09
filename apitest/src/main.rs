use reqwest::Error;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use tokio::runtime::Runtime;
use serde_json::Value;

fn main() {
    let mut rt = Runtime::new().unwrap();

    rt.block_on(async {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", "YOUR_BEARER_TOKEN")).unwrap());
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        let res = client.get("https://api.spotinst.io/setup/account")
            .send()
            .await
            .unwrap();

        let text = res.text().await.unwrap();
        let v: Value = serde_json::from_str(&text).unwrap();
        println!("{:#?}", v);
    });
}