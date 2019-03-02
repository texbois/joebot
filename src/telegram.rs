use serde_derive::Deserialize;

pub mod long_poll;

pub type ClientResult<T> = Result<T, reqwest::Error>;

#[derive(Debug)]
pub enum Message {
    Command { sender: String, command: String, receiver: Option<String> },
    Text { sender: String, contents: String }
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

    pub fn poll_messages(&self) -> long_poll::MessagePoller {
        long_poll::MessagePoller::new(self)
    }

    fn api_method_get(&self, method: &str, query: &[(&str, String)]) -> reqwest::RequestBuilder {
        self.client
            .get(&format!("{}/{}", self.api_root, method))
            .query(query)
    }
}
