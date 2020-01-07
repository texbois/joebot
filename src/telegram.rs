use crate::JoeResult;
use serde_json::json;

mod long_poll;

#[derive(Debug)]
pub enum MessageContents {
    Text(String),
    Command {
        command: String,
        receiver: Option<String>,
    },
}

#[derive(Debug)]
pub struct Message {
    pub chat_id: i64,
    pub sender: String,
    pub contents: MessageContents,
}

pub struct Telegram {
    client: reqwest::blocking::Client,
    api_root: String,
}

impl Telegram {
    pub fn new(token: &str) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            api_root: format!("https://api.telegram.org/bot{}/", token),
        }
    }

    pub fn get_bot_username(&self) -> JoeResult<String> {
        let mut resp: serde_json::Value = self.api_method("getMe", None).send()?.json()?;
        match resp["result"].take()["username"].take() {
            serde_json::Value::String(username) => Ok(username),
            _ => Err(format!("get_bot_username: unexpected response {}", resp).into()),
        }
    }

    pub fn send_message(&self, chat_id: i64, text: &str) -> JoeResult<()> {
        self.api_method(
            "sendMessage",
            Some(json!({
                "chat_id": chat_id,
                "text": text
            })),
        )
        .send()?;
        Ok(())
    }

    pub fn poll_messages<F: FnMut(Message) -> JoeResult<()>>(&self, callback: F) -> JoeResult<()> {
        long_poll::do_poll(&self, callback)
    }

    fn api_method(
        &self,
        method: &str,
        payload: Option<serde_json::Value>,
    ) -> reqwest::blocking::RequestBuilder {
        let req = self.client.get(&[&self.api_root, method].concat());

        match payload {
            Some(json) => req.json(&json),
            None => req,
        }
    }
}
