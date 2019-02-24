use serde_derive::Deserialize;
use serde_json::json;

pub type VkResult<T> = Result<T, reqwest::Error>;

#[derive(Deserialize, Debug)]
pub struct VkUser {
    pub screen_name: String,
    pub first_name: String,
    pub last_name: String
}

#[derive(Deserialize, Debug)]
pub struct VkPollState {
    server: String,
    key: String,
    ts: u64
}

pub struct Vk {
    token: String,
    chat_id: u64,
    client: reqwest::Client
}

impl Vk {
    pub fn new(token: String, chat_id: String) -> Self {
        Self {
            token,
            chat_id: chat_id.parse().unwrap(),
            client: reqwest::Client::new()
        }
    }

    pub fn init_long_poll(&self) -> VkResult<VkPollState> {
        let mut resp: serde_json::Value = self
            .api_get_method("messages.getLongPollServer", &[("lp_version", "2".to_owned())])
            .send()?
            .json()?;

        Ok(serde_json::from_value(resp["response"].take()).unwrap())
    }

    pub fn get_chat_members(&self) -> VkResult<Vec<VkUser>> {
        let mut resp: serde_json::Value = self
            .api_get_method(
                "messages.getConversationMembers",
                &[
                    ("peer_id", format!("{}", 2000000000 + self.chat_id)),
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
