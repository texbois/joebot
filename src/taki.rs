use crate::{messages, storage, vk::{VkUser, VkMessage, VkMessageOrigin}};
use rand::seq::SliceRandom;

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

    fn pick_random_target() -> (&'static str, Vec<&'static str>) {
        let mut rng = rand::thread_rng();
        let name = messages::SCREEN_NAMES.choose(&mut rng).unwrap();
        let messages: Vec<&str> = messages::get_by_name(name).unwrap().choose_multiple(&mut rng, 5).cloned().collect();

        (name, messages)
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

        match text {
            "начнем" if self.ongoing.unwrap().name == "" => {
                let (name, messages) = self.pick_random_target(); // doesnt work for some reason
                self.ongoing = Some(OngoingGame { name, guesses: 0 });
                Some((message.origin, "Начнем игру!".to_owned()))
            },
            "начнем" => None,
            "статы" => {
                let stats = self.storage.fetch_sorted_set(&"set");
                Some((message.origin, "Статы".to_owned()))

            }
            _ => {
                let limit: u8 = 5;
                let mention = text.split(" ").into_iter().nth(0).unwrap();
                if mention == self.ongoing.unwrap().name {
                    self.storage.incr_in_set(&"set", mention, (limit - self.ongoing.unwrap().guesses) as i32);
                    self.ongoing = None;
                    Some((message.origin, "Поздравляю!".to_owned()))
                }
                else {
                    let res = Some((message.origin, "Не угадал!".to_owned()));
                    self.ongoing.unwrap().guesses += 1;
                    if self.ongoing.unwrap().guesses == 5 {
                        self.ongoing = Some(OngoingGame { name: "", guesses: 0 });
                        res = Some((message.origin, "Игра окончена".to_owned()));
                    }
                    res
                }
            }
        }

        //Some((message.origin, "nice".to_owned()))
    }
}

