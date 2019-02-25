use serde_derive::Deserialize;
use serde_json::json;

pub type VkResult<T> = Result<T, reqwest::Error>;

#[derive(Deserialize, Debug)]
pub struct VkUser {
    pub screen_name: String,
    pub first_name: String,
    pub last_name: String,
    pub id: u64
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

pub struct VkMessage {
    text: String,
    from_id: u64
}

impl Vk {
    pub fn new(token: String, chat_id: String) -> Self {
        Self {
            token,
            chat_id: chat_id.parse().unwrap(),
            client: reqwest::Client::new()
        }
    }

    pub fn poll_for_messages(&self, poll_state: VkPollState) -> VkResult<(Vec<VkMessage>, VkPollState)> {
        const MESSAGE_EVENT: u64 = 0;

        let VkPollState { server, key, mut ts } = poll_state;

        let mut resp: serde_json::Value = self.client
            .get(&format!("https://{}?act=a_check&key={}&ts={}&wait=25&mode=2&version=2", server, key, ts))
            .send()?
            .json()?;

        ts = resp["ts"].as_u64().unwrap();

        let messages: Vec<VkMessage> = resp["updates"].as_array().unwrap()
            .into_iter()
            .filter_map(|u| u.as_array())
            .filter(|uf| uf[0] == MESSAGE_EVENT && uf[3] == 2000000000 + self.chat_id)
            .map(|uf| VkMessage { text: uf[6].as_str().unwrap().to_owned(), from_id: uf[6]["from"].as_u64().unwrap() })
            .collect();

        Ok((messages, VkPollState { server, key, ts }))
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
