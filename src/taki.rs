use crate::{messages, storage, telegram::Message};
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
    pub fn new(chat_id: i64, redis: &'a storage::Redis) -> Self {
        Self {
            ongoing: None,
            storage: redis.get_game_storage("taki", chat_id),
            rng: SmallRng::from_entropy()
        }
    }

    pub fn process_with_reply(&mut self, message: &Message) -> Option<String> {
        use crate::telegram::MessageContents::*;

        match (&message.contents, &mut self.ongoing) {
            (&Command { ref command, .. }, None) if command == "takistart" => {
                let ((screen_name, full_name, full_name_trunc), messages) = pick_random_target(&mut self.rng);

                self.ongoing = Some(OngoingGame {
                    screen_name,
                    full_name,
                    full_name_trunc,
                    guesses: 0
                });

                Some(format!("{}\n\n* {}", START_MESSAGES.choose(&mut self.rng).unwrap(), messages.join("\n* ")))
            },
            (&Command { ref command, .. }, _) if command == "stats" => {
                let stats = self.storage.fetch_sorted_set("scores").unwrap()
                    .into_iter().enumerate()
                    .map(|(index, (name, score))| format!("{}) {} -- {}", index + 1, name, score))
                    .collect::<Vec<_>>()
                    .join("\n");

                Some(format!("Статы:\n{}", stats))
            },
            (&Command { ref command, .. }, _) if command == "suspects" => {
                let suspects = messages::SCREEN_NAMES.iter()
                    .enumerate()
                    .zip(messages::FULL_NAMES.iter())
                    .zip(messages::FULL_NAMES_TRUNC.iter())
                    .map(|(((idx, screen_name), full_name), full_name_trunc)|
                         format!("{}) {} под псевдонимами \"{}\", \"{}\"", idx + 1, full_name, screen_name, full_name_trunc)
                    )
                    .collect::<Vec<String>>()
                    .join("\n");

                Some(format!("Подозреваемые:\n{}", suspects))
            },
            (&Text(ref text), Some(ref mut game)) => {
                let first_sep = text.find(' ').unwrap_or(text.len() - 1);
                let extracted_screen_name: String = text.chars().take(first_sep).collect();
                let extracted_full_name_trunc: String = text.chars().take(first_sep + 2).collect();

                let has_matched = extracted_screen_name == game.screen_name ||
                    extracted_full_name_trunc == game.full_name_trunc;

                if has_matched {
                    let score = (MAX_GUESSES - game.guesses) as i32;
                    self.storage.incr_in_set("scores", &message.sender, score).unwrap();
                    self.ongoing = None;

                    Some(format!("{}\n{} +{}", WIN_MESSAGES.choose(&mut self.rng).unwrap(), message.sender, score))
                }
                else {
                    game.guesses += 1;

                    if game.guesses == MAX_GUESSES {
                        let reply = format!("{}\nЭто был {} ({})", LOSE_MESSAGES.choose(&mut self.rng).unwrap(), game.full_name, game.screen_name);
                        self.ongoing = None;

                        Some(reply)
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
