use crate::{messages, storage, vk::{VkUser, VkMessage, VkMessageOrigin}};
use rand::{FromEntropy, seq::SliceRandom, rngs::SmallRng};

const MAX_GUESSES: u8 = 5;
const MESSAGES_SHOWN: usize = 3;
const START_MESSAGES: [&'static str; 3] = [
    "Начнем игру! Вычислите дружка-пирожка по цитаткам:",
    "Поехали, други! Три цитаты, один чувак из чата - вы знаете, что делать:",
    "The game must go on! Вычисли приятеля по айпи:"
];
const WIN_MESSAGES: [&'static str; 3] = [
    "Хорошая работа, приятель.",
    "Ты справился? Неплохо, дружище.",
    "Дело сделано, дружочки."
];
const LOSE_MESSAGES: [&'static str; 4] = [
    "С меня хватит, уроды. В следующий раз удачи!",
    "Очень жаль, но вы не справились. Я закрываю игру.",
    "Я вас выслушал, товарищи студенты... А теперь игра окончена.",
    "Wake up, Neo. You obosralsya. Game over."
];

pub struct Taki<'a> {
    chat_id: u64,
    bot_user_id: u64,
    players: Vec<VkUser>,
    ongoing: Option<OngoingGame>,
    storage: storage::ChatGameStorage<'a>,
    rng: SmallRng
}

struct OngoingGame {
    screen_name: &'static str,
    full_name: &'static str,
    full_name_trunc: &'static str,
    guesses: u8
}

impl<'a> Taki<'a> {
    pub fn new(chat_id: u64, bot_user: &VkUser, players: Vec<VkUser>, redis: &'a storage::Redis) -> Self {
        Self {
            chat_id,
            players,
            bot_user_id: bot_user.id,
            ongoing: None,
            storage: redis.get_game_storage("taki", chat_id),
            rng: SmallRng::from_entropy()
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
        let text: String = if is_bot_mentioned {
            let mention_end = message.text.find(']').unwrap_or(0);
            message.text[mention_end + 1..].trim().to_lowercase()
        }
        else {
            message.text.trim().to_lowercase()
        };

        match (text.as_str(), &mut self.ongoing) {
            ("начнем", None) if is_bot_mentioned => {
                let ((screen_name, full_name, full_name_trunc), messages) = pick_random_target(&mut self.rng);

                self.ongoing = Some(OngoingGame {
                    screen_name,
                    full_name,
                    full_name_trunc,
                    guesses: 0
                });

                let start_message = format!("{}\n\n* {}", START_MESSAGES.choose(&mut self.rng).unwrap(), messages.join("\n* "));
                Some((message.origin, start_message))
            },
            ("статы", _) if is_bot_mentioned => {
                let stats = self.storage.fetch_sorted_set("scores").unwrap()
                    .into_iter().enumerate()
                    .map(|(index, (name, score))| format!("{}) {} -- {}", index + 1, name, score))
                    .collect::<Vec<_>>()
                    .join("\n");
                Some((message.origin, format!("Статы:\n{}", stats)))
            },
            ("подозреваемые", _) if is_bot_mentioned => {
                let suspects = messages::SCREEN_NAMES.iter()
                    .enumerate()
                    .zip(messages::FULL_NAMES.iter())
                    .zip(messages::FULL_NAMES_TRUNC.iter())
                    .map(|(((idx, screen_name), full_name), full_name_trunc)|
                         format!("{}) {} под псевдонимами \"{}\", \"{}\"", idx + 1, full_name, screen_name, full_name_trunc)
                    )
                    .collect::<Vec<String>>()
                    .join("\n");

                Some((message.origin, format!("Подозреваемые:\n{}", suspects)))
            },
            (text, Some(ref mut game)) => {
                let first_sep = text.find(' ').unwrap_or(text.len() - 1);
                let extracted_screen_name: String = text.chars().take(first_sep).collect();
                let extracted_full_name_trunc: String = text.chars().take(first_sep + 2).collect();

                let has_matched = extracted_screen_name == game.screen_name ||
                    extracted_full_name_trunc == game.full_name_trunc;

                if has_matched {
                    let score = (MAX_GUESSES - game.guesses) as i32;
                    let name = format!("{} {}", sender.first_name, sender.last_name);
                    self.storage.incr_in_set("scores", &name, score);
                    self.ongoing = None;
                    Some((message.origin, format!("{}\n{} +{}", WIN_MESSAGES.choose(&mut self.rng).unwrap(), name, score)))
                }
                else {
                    game.guesses += 1;
                    if game.guesses == MAX_GUESSES {
                        let reply = format!("{}\nЭто был {} ({})", LOSE_MESSAGES.choose(&mut self.rng).unwrap(), game.full_name, game.screen_name);
                        self.ongoing = None;
                        Some((message.origin, reply))
                    }
                    else {
                        None
                    }
                }
            },
            _ => None
        }
    }
}

fn pick_random_target(rng: &mut SmallRng) -> ((&'static str, &'static str, &'static str), Vec<&'static str>) {
    let screen_name = messages::SCREEN_NAMES.choose(rng).unwrap();

    let (full_name, full_name_trunc, all_messages) =
        messages::get_full_name_full_name_trunc_messages(screen_name).unwrap();

    let message_sample: Vec<&str> = all_messages
        .choose_multiple(rng, MESSAGES_SHOWN)
        .cloned().collect();

    ((screen_name, full_name, full_name_trunc), message_sample)
}
