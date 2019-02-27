use crate::{messages, storage, vk::{VkUser, VkMessage, VkMessageOrigin}};

pub struct Taki<'a> {
    chat_id: u64,
    bot_user_id: u64,
    players: Vec<VkUser>,
    ongoing: Option<OngoingGame>,
    storage: storage::ChatGameStorage<'a>
}

struct OngoingGame {
    name: &'static str,
    guesses: u8
}

impl<'a> Taki<'a> {
    pub fn new(chat_id: u64, bot_user: &VkUser, players: Vec<VkUser>, redis: &'a storage::Redis) -> Self {
        Self {
            chat_id,
            players,
            bot_user_id: bot_user.id,
            ongoing: None,
            storage: redis.get_game_storage("taki", chat_id)
        }
    }

    pub fn process_with_reply(&mut self, message: &VkMessage) -> Option<(VkMessageOrigin, String)> {
        let sender: &VkUser = match message.origin {
            VkMessageOrigin::Chatroom(message_chat_id) if self.chat_id == message_chat_id =>
                self.players.iter().find(|u| u.id == message.sender_id),
            _ =>
                None
        }?;

        let is_mention = message.text.starts_with(&format!("[id{}|", self.bot_user_id));
        let text = if is_mention {
            let mention_end = message.text.find(']').unwrap_or(0);
            &message.text[mention_end + 1..]
        }
        else {
            &message.text
        };

        Some((message.origin, "nice".to_owned()))
    }
}
