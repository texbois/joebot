use crate::{messages, storage, vk::{VkUser, VkMessage, VkMessageOrigin}};
use rand::seq::SliceRandom;

const MAX_GUESSES: u8 = 5;
const MESSAGES_SHOWN: usize = 3;

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

        let is_bot_mentioned = message.text.starts_with(&format!("[id{}|", self.bot_user_id));
        let text = if is_bot_mentioned {
            let mention_end = message.text.find(']').unwrap_or(0);
            &message.text[mention_end + 1..]
        }
        else {
            &message.text
        };

        match (text, &mut self.ongoing) {
            ("начнем", None) if is_bot_mentioned => {
                let (name, messages) = pick_random_target();
                self.ongoing = Some(OngoingGame { name, guesses: 0 });
                Some((message.origin, "Начнем игру!".to_owned()))
            },
            ("статы", _) if is_bot_mentioned => {
                let stats = self.storage.fetch_sorted_set(&"set");
                Some((message.origin, "Статы".to_owned()))
            }
            (text, Some(ref mut game)) => {
                let mention = text.split(" ").into_iter().nth(0).unwrap();

                if mention == game.name {
                    self.storage.incr_in_set(&"set", mention, (MAX_GUESSES - game.guesses) as i32);
                    self.ongoing = None;
                    Some((message.origin, "Поздравляю!".to_owned()))
                }
                else {
                    game.guesses += 1;
                    if game.guesses == MAX_GUESSES {
                        self.ongoing = None;
                        Some((message.origin, "Игра окончена".to_owned()))
                    }
                    else {
                        Some((message.origin, "Не угадал!".to_owned()))
                    }
                }
            },
            _ => None
        }
    }
}

fn pick_random_target() -> (&'static str, Vec<&'static str>) {
    let mut rng = rand::thread_rng();
    let name = messages::SCREEN_NAMES.choose(&mut rng).unwrap();
    let messages: Vec<&str> = messages::get_by_name(name).unwrap()
        .choose_multiple(&mut rng, MESSAGES_SHOWN)
        .cloned().collect();

    (name, messages)
}
