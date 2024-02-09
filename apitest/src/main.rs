use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use tokio::runtime::Runtime;
use serde_json::Value;
use std::error::Error;
use csv::Writer;
use std::collections::HashMap;

fn main() {
    let mut rt = Runtime::new().unwrap();

    rt.block_on(async {
        let url = "https://api.spotinst.io/setup/account";
        let token = "<<bearer token>>";
        let response = make_request(url, token).await.unwrap();
        println!("{:#?}", response);

        // Assuming the response is an object with a "response" field that is an object
        // with an "items" field that is an array of objects
        if let Value::Object(obj) = &response {
            if let Some(Value::Object(response_obj)) = obj.get("response") {
                if let Some(Value::Array(array)) = response_obj.get("items") {
                    println!("Found an 'items' array. Writing to CSV file...");

                    let file_path = "output.csv";
                    match Writer::from_path(file_path) {
                        Ok(mut wtr) => {
                            // Write the headers
                            if let Some(Value::Object(first_obj)) = array.first() {
                                let headers: Vec<&str> = first_obj.keys().map(AsRef::as_ref).collect();
                                if let Err(e) = wtr.write_record(&headers) {
                                    println!("Error writing headers: {}", e);
                                }
                            }

                            // Write the values
                            for value in array {
                                if let Value::Object(obj) = value {
                                    let values: Vec<String> = obj.values().map(|v| v.to_string()).collect();
                                    if let Err(e) = wtr.write_record(&values) {
                                        println!("Error writing record: {}", e);
                                    }
                                }
                            }

                            if let Err(e) = wtr.flush() {
                                println!("Error flushing writer: {}", e);
                            }
                        },
                        Err(e) => {
                            println!("Error creating writer: {}", e);
                        }
                    }
                } else {
                    println!("'items' field is not an array.");
                }
            } else {
                println!("No 'response' field found.");
            }
        } else {
            println!("Response is not an object.");
        }
    });
}

async fn make_request(url: &str, token: &str) -> Result<Value, Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", token))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let res = client.get(url)
        .send()
        .await?;

    let text = res.text().await?;
    let v: Value = serde_json::from_str(&text)?;

    Ok(v)
}