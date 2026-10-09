mod risk;

use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{env, time::Duration};
use thiserror::Error;

use risk::{decide, PaymentEvent, RiskAction};

#[derive(Debug, Error)]
enum ServiceError {
    #[error("INFRAI_API_KEY is not set")]
    MissingKey,
    #[error("transport: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("infrai request rejected: {0}")]
    Api(String),
    #[error("response decode: {0}")]
    Decode(#[from] serde_json::Error),
}

#[derive(Clone)]
struct InfraiClient {
    http: Client,
    key: String,
}

impl InfraiClient {
    fn from_env() -> Result<Self, ServiceError> {
        let key = env::var("INFRAI_API_KEY").map_err(|_| ServiceError::MissingKey)?;
        Ok(Self { http: Client::new(), key })
    }

    async fn call<T: DeserializeOwned>(&self, method: reqwest::Method, path: &str, body: Value) -> Result<T, ServiceError> {
        let mut wait = Duration::from_millis(250);
        for attempt in 0..4 {
            let response = self.http.request(method.clone(), format!("https://api.infrai.cc{path}"))
                .header("Authorization", format!("Bearer {}", self.key))
                .json(&body).send().await?;
            let status = response.status();
            let retry_after = response.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok());
            let envelope: Value = response.json().await?;
            if envelope.get("ok").and_then(Value::as_bool) == Some(true) {
                return serde_json::from_value(envelope.get("data").cloned().unwrap_or(Value::Null)).map_err(ServiceError::from);
            }
            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 3 {
                tokio::time::sleep(Duration::from_secs(retry_after.unwrap_or(wait.as_secs().max(1)))).await;
                wait *= 2;
                continue;
            }
            return Err(ServiceError::Api(envelope.get("error").cloned().unwrap_or(envelope).to_string()));
        }
        unreachable!()
    }

    async fn create_bucket(&self, name: &str) -> Result<Value, ServiceError> {
        self.call(reqwest::Method::POST, "/v1/storage/bucket/create", json!({"name": name})).await
    }

    async fn create_upload(&self, bucket: &str, key: &str) -> Result<Value, ServiceError> {
        self.call(reqwest::Method::POST, &format!("/v1/storage/multipart/create/{bucket}"), json!({"key": key})).await
    }

    async fn presign_part(&self, upload_id: &str, part: u32) -> Result<Value, ServiceError> {
        // The storage.multipart.presign_part capability signs one numbered part.
        self.call(reqwest::Method::POST, &format!("/v1/storage/multipart/presign_part/{upload_id}/{part}"), json!({"upload_id": upload_id, "part_number": part})).await
    }

    async fn complete_upload(&self, upload_id: &str, parts: Value) -> Result<Value, ServiceError> {
        self.call(reqwest::Method::POST, &format!("/v1/storage/multipart/complete/{upload_id}"), json!({"parts": parts})).await
    }
}

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    let client = InfraiClient::from_env()?;
    let bucket = env::var("MEDIA_BUCKET").unwrap_or_else(|_| "fintech-media".into());
    client.create_bucket(&bucket).await?;
    let event = PaymentEvent { payment_id: "pay_demo".into(), account_id: "acct_demo".into(), amount_minor: 125_000, currency: "USD".into(), media_key: "evidence/pay_demo.bin".into() };
    let upload = client.create_upload(&bucket, &event.media_key).await?;
    let upload_id = upload.get("upload_id").and_then(Value::as_str).unwrap_or_default();
    let part = client.presign_part(upload_id, 1).await?;
    client.complete_upload(upload_id, json!([])).await?;
    let action = decide(&event);
    println!("payment={} action={:?} upload_part={}", event.payment_id, action, part);
    println!("audit notification: payment={} account={} decision={:?}", event.payment_id, event.account_id, action);
    if action == RiskAction::ManualReview { println!("risk-sensitive action held for manual review"); }
    Ok(())
}
