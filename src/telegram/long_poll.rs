use std::collections::VecDeque;
use serde_json::json;

use crate::telegram::{Telegram, Message, MessageContents};

pub struct MessagePoller<'a> {
    client: &'a Telegram,
    message_queue: VecDeque<Message>,
    update_offset: u64
}

impl<'a> Iterator for MessagePoller<'a> {
    type Item = Message;

    fn next(&mut self) -> Option<Message> {
        while self.message_queue.is_empty() {
            self.poll_updates();
        }
        self.message_queue.pop_front()
    }
}

impl<'a> MessagePoller<'a> {
    pub fn new(client: &'a Telegram) -> Self {
        Self { client, message_queue: VecDeque::new(), update_offset: 0 }
    }

    fn poll_updates(&mut self) {
        let resp: serde_json::Value = self.client
            .api_method("getUpdates", Some(json!({
                "timeout": 25,
                "allowed_updates": ["message"],
                "offset": self.update_offset + 1
            })))
            .send().unwrap()
            .json().unwrap();

        if let Some(last_update_id) = resp["result"].as_array().and_then(|u| u.last()).and_then(|u| u["update_id"].as_u64()) {
            self.update_offset = last_update_id;
        }
   
        self.message_queue.extend(resp["result"].as_array().unwrap()
            .into_iter().filter_map(parse_text_message));
    }
}

fn parse_text_message(update_obj: &serde_json::Value) -> Option<Message> {
    let message_obj = update_obj.get("message")?;

    let chat_id = message_obj.get("chat")?.get("id")?.as_i64()?;
    let text = message_obj.get("text")?.as_str()?;
    /* If the current message contains a "text" field, it also has { from: { username: "..." } } */
    let sender = message_obj["from"]["username"].as_str()?.to_owned();

    let bot_command = message_obj.get("entities")
        .and_then(|es| es.as_array())
        .and_then(|es| es.iter().find(|e| e["type"] == "bot_command" && e["offset"] == 0))
        .and_then(|e| {
            let cmd_len = e["length"].as_u64().unwrap() as usize;

            match &text[1 /* skip forward slash */..cmd_len].split('@').collect::<Vec<_>>()[..] {
                &[cmd] => Some((cmd.to_owned(), None)),
                &[cmd, receiver] => Some((cmd.to_owned(), Some(receiver.to_owned()))),
                _ => None
            }
        });

    let contents = if let Some((command, receiver)) = bot_command {
        MessageContents::Command { command, receiver }
    }
    else {
        MessageContents::Text(text.to_owned())
    };

    Some(Message { chat_id, sender, contents })
}
