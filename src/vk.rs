use serde_derive::Deserialize;

pub mod long_poll;

pub type VkResult<T> = Result<T, reqwest::Error>;

#[derive(Deserialize, Debug)]
pub struct VkUser {
    pub screen_name: String,
    pub first_name: String,
    pub last_name: String,
    pub id: u64
}

#[derive(Debug)]
pub enum VkMessageOrigin {
    Chatroom(u64),
    User(u64)
}

#[derive(Debug)]
pub struct VkMessage {
    origin: VkMessageOrigin,
    text: String,
    sender_id: u64
}

pub struct Vk {
    token: String,
    client: reqwest::Client
}

impl Vk {
    pub fn new(token: String) -> Self {
        Self { token, client: reqwest::Client::new() }
    }

    pub fn poll_messages(&self) -> VkResult<long_poll::VkMessagePoller> {
        let mut resp: serde_json::Value = self
            .api_get_method("messages.getLongPollServer", &[("lp_version", "2".to_owned())])
            .send()?
            .json()?;

        Ok(long_poll::VkMessagePoller::new(self, resp["response"].take()))
    }

    pub fn get_chat_members(&self, chat_id: u64) -> VkResult<Vec<VkUser>> {
        let mut resp: serde_json::Value = self
            .api_get_method(
                "messages.getConversationMembers",
                &[
                    ("peer_id", format!("{}", 2000000000 + chat_id)),
                    ("fields", "screen_name,first_name,last_name".to_owned())
                ]
            )
            .send()?
            .json()?;

        let users: Vec<VkUser> =
            serde_json::from_value(resp["response"]["profiles"].take()).unwrap();

        Ok(users)
    }

    fn api_get_method(&self, method: &str, query: &[(&str, String)]) -> reqwest::RequestBuilder {
        self.client
            .get(&format!("https://api.vk.com/method/{}", method))
            .query(query)
            .query(&[
                ("v", "5.92".to_owned()),
                ("access_token", self.token.to_owned())
            ])
    }
}
