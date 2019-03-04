use serde_json::json;

pub mod long_poll;

pub type ClientResult<T> = Result<T, reqwest::Error>;

#[derive(Debug)]
pub enum MessageContents {
    Text(String),
    Command { command: String, receiver: Option<String> }
}

#[derive(Debug)]
pub struct Message {
    pub chat_id: i64,
    pub sender: String,
    pub contents: MessageContents
}

pub struct Telegram {
    client: reqwest::Client,
    api_root: String
}

impl Telegram {
    pub fn new(token: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_root: format!("https://api.telegram.org/bot{}/", token)
        }
    }

    pub fn get_bot_username(&self) -> ClientResult<String> {
        let resp: serde_json::Value = self.api_method("getMe", None).send()?.json()?;
        Ok(resp["result"]["username"].as_str().unwrap().to_owned())
    }

    pub fn send_message(&self, chat_id: i64, text: &str) -> ClientResult<()> {
        self.api_method("sendMessage", Some(json!({
            "chat_id": chat_id,
            "text": text
        }))).send()?;
        Ok(())
    }

    pub fn poll_messages(&self) -> long_poll::MessagePoller {
        long_poll::MessagePoller::new(self)
    }

    fn api_method(&self, method: &str, payload: Option<serde_json::Value>) -> reqwest::RequestBuilder {
        let req = self.client.get(&[&self.api_root, method].concat());

        match payload {
            Some(json) => req.json(&json),
            None => req
        }
    }
}
