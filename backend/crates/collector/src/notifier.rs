//! Server酱通知器。POST https://sct.ftqq.com/{sendkey}.send

use async_trait::async_trait;
use domain::notifier::{Notifier, NotifyError};

pub struct ServerChanNotifier {
    client: reqwest::Client,
    sendkey: String,
    base_url: String,
}

impl ServerChanNotifier {
    pub fn new(client: reqwest::Client, sendkey: String) -> Self {
        Self {
            client,
            sendkey,
            base_url: "https://sct.ftqq.com".into(),
        }
    }

    /// 测试用：注入自定义 base_url（指向 wiremock）。
    #[cfg(test)]
    pub fn with_base_url(client: reqwest::Client, sendkey: String, base_url: String) -> Self {
        Self {
            client,
            sendkey,
            base_url,
        }
    }
}

#[async_trait]
impl Notifier for ServerChanNotifier {
    async fn send(&self, title: &str, desp: &str) -> Result<(), NotifyError> {
        let url = format!("{}/{}.send", self.base_url, self.sendkey);
        let resp = self
            .client
            .post(&url)
            .form(&[("title", title), ("desp", desp)])
            .send()
            .await
            .map_err(|e| NotifyError::Http(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(NotifyError::Http(format!("serverchan status {status}")));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotifyError::Http(e.to_string()))?;
        let code = body.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        if code != 0 {
            let message = body
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown")
                .to_string();
            return Err(NotifyError::Api { code, message });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn send_success_returns_ok() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/SCT123.send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": 0,
                "message": "ok"
            })))
            .mount(&server)
            .await;

        let n = ServerChanNotifier::with_base_url(
            reqwest::Client::new(),
            "SCT123".into(),
            server.uri(),
        );
        n.send("标题", "正文").await.unwrap();
    }

    #[tokio::test]
    async fn send_api_error_returns_api_variant() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/SCT123.send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": 40001,
                "message": "bad sendkey"
            })))
            .mount(&server)
            .await;

        let n = ServerChanNotifier::with_base_url(
            reqwest::Client::new(),
            "SCT123".into(),
            server.uri(),
        );
        let err = n.send("标题", "正文").await.unwrap_err();
        match err {
            NotifyError::Api { code, message } => {
                assert_eq!(code, 40001);
                assert_eq!(message, "bad sendkey");
            }
            other => panic!("expected Api, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn send_http_error_on_5xx() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/SCT123.send"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let n = ServerChanNotifier::with_base_url(
            reqwest::Client::new(),
            "SCT123".into(),
            server.uri(),
        );
        let err = n.send("标题", "正文").await.unwrap_err();
        assert!(matches!(err, NotifyError::Http(_)));
    }
}
