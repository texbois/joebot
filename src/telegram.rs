use serde_derive::Deserialize;

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
            api_root: format!("https://api.telegram.org/bot{}", token)
        }
    }

    pub fn get_bot_username(&self) -> ClientResult<String> {
        let resp: serde_json::Value = self.api_method("getMe", &[]).send()?.json()?;
        Ok(resp["result"]["username"].as_str().unwrap().to_owned())
    }

    pub fn poll_messages(&self) -> long_poll::MessagePoller {
        long_poll::MessagePoller::new(self)
    }

    fn api_method(&self, method: &str, query: &[(&str, String)]) -> reqwest::RequestBuilder {
        self.client
            .get(&format!("{}/{}", self.api_root, method))
            .query(query)
    }
}
