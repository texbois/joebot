use std::collections::VecDeque;
use serde_derive::Deserialize;

use crate::vk::{Vk, VkMessage, VkMessageOrigin};

#[derive(Deserialize, Debug)]
struct VkPollState {
    server: String,
    key: String,
    ts: u64
}

pub struct VkMessagePoller<'a> {
    vk: &'a Vk,
    poll_state: VkPollState,
    message_queue: VecDeque<VkMessage>
}

impl<'a> Iterator for VkMessagePoller<'a> {
    type Item = VkMessage;

    fn next(&mut self) -> Option<VkMessage> {
        while self.message_queue.is_empty() {
            self.poll_updates();
        }
        self.message_queue.pop_front()
    }
}

impl<'a> VkMessagePoller<'a> {
    pub fn new(vk: &'a Vk, poll_server_json: serde_json::Value) -> Self {
        let poll_state = serde_json::from_value(poll_server_json).unwrap();
        let message_queue = VecDeque::new();

        Self { vk, poll_state, message_queue }
    }

    fn poll_updates(&mut self) {
        let mut resp: serde_json::Value = self.vk.client
            .get(&format!(
                "https://{}?act=a_check&key={}&ts={}&wait=25&mode=2&version=2",
                self.poll_state.server, self.poll_state.key, self.poll_state.ts
            ))
            .send().unwrap()
            .json().unwrap();

        self.poll_state.ts = resp["ts"].as_u64().unwrap();
        self.message_queue.extend(resp["updates"].as_array().unwrap()
            .into_iter().filter_map(try_parse_message));
    }
}

fn try_parse_message(event_json: &serde_json::Value) -> Option<VkMessage> {
    const MESSAGE_EVENT: u64 = 4;

    let event = event_json.as_array()?;

    if event[0] != MESSAGE_EVENT {
        return None
    }

    let text = event[5].as_str()?.to_owned();
    let origin_id = event[3].as_u64()?;

    if origin_id >= 2000000000 {
        let origin = VkMessageOrigin::Chatroom(origin_id - 2000000000);
        let sender_id = event[6]["from"].as_str()?.parse().ok()?;

        Some(VkMessage { text, origin, sender_id })
    }
    else {
        let origin = VkMessageOrigin::User(origin_id);

        Some(VkMessage { text, origin, sender_id: origin_id })
    }
}
