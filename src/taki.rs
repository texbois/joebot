use crate::{
    messages::{self, MessageDump},
    storage, telegram,
};
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use crate::HandlerResult;

const INIT_SCORE: i32 = 5;
const MESSAGES_SHOWN: usize = 3;
const START_MESSAGES: [(&'static str, &'static str); 3] = [
    ("Один мудрец сказал:", "Кто же это был?"),
    (
        "Последний раз подозреваемого видели в местном баре, где он произнес:",
        "Найдите мне этого пса!",
    ),
    ("Дружок-пирожок оставил вам послание:", "Узнали?"),
];
const WIN_MESSAGES: [&'static str; 3] = [
    "Хорошая работа, дружище.",
    "А ты неплох, приятель.",
    "Дело сделано, джентельмены.",
];
const LOSE_MESSAGES: [&'static str; 4] = [
    "Казино не взломано.",
    "Игра закрыта, неудачники.",
    "Очень жаль, но вы проиграли.",
    "Удачи в другой раз, амигос.",
];

pub struct Taki<'a> {
    messages: &'a MessageDump,
    storage: storage::ChatGameStorage<'a>,
    ongoing: Option<OngoingGame<'a>>,
    rng: SmallRng,
}

struct OngoingGame<'a> {
    suspect: &'a messages::Author,
    score: i32,
}

impl<'a> Taki<'a> {
    pub fn new(messages: &'a MessageDump, chat_id: i64, redis: &'a mut storage::Redis) -> Self {
        Self {
            messages,
            storage: redis.get_game_storage("taki", chat_id),
            ongoing: None,
            rng: SmallRng::from_entropy(),
        }
    }

    pub fn handle_message(&mut self, message: &telegram::Message) -> HandlerResult {
        use crate::telegram::MessageContents::*;

        match (&message.contents, &mut self.ongoing) {
            (&Command { ref command, .. }, None) if command == "takistart" => {
                let (suspect, messages) = pick_random_suspect(self.messages, &mut self.rng);

                self.ongoing = Some(OngoingGame {
                    suspect,
                    score: INIT_SCORE,
                });
                let (start_prefix, start_suffix) = START_MESSAGES.choose(&mut self.rng).unwrap();

                HandlerResult::Response(format!(
                    "{}\n\n* {}\n\n{}",
                    start_prefix,
                    messages.join("\n* "),
                    start_suffix
                ))
            }
            (&Command { ref command, .. }, _) if command == "takistats" => {
                let stats = self
                    .storage
                    .fetch_sorted_set("scores")
                    .unwrap()
                    .into_iter()
                    .enumerate()
                    .map(|(index, (name, score))| format!("{}) {} -- {}", index + 1, name, score))
                    .collect::<Vec<_>>()
                    .join("\n");

                HandlerResult::Response(format!("Статы:\n{}", stats))
            }
            (&Command { ref command, .. }, _) if command == "takisuspects" => {
                let suspects = list_suspects(self.messages).join("\n");
                HandlerResult::Response(format!("Подозреваемые:\n{}", suspects))
            }
            (&Text(ref text), Some(ref mut game)) => {
                let text_lower = text.to_lowercase();

                if text_lower == game.suspect.short_name || text_lower == game.suspect.full_name {
                    let reply = format!(
                        "{}\n{} +{}",
                        WIN_MESSAGES.choose(&mut self.rng).unwrap(),
                        message.sender,
                        game.score
                    );

                    self.storage
                        .incr_in_set("scores", &message.sender, game.score)
                        .unwrap();
                    self.ongoing = None;

                    HandlerResult::Response(reply)
                } else {
                    game.score -= 1;

                    if game.score == 0 {
                        let reply = format!(
                            "{}\nЭто был {} ({})",
                            LOSE_MESSAGES.choose(&mut self.rng).unwrap(),
                            game.suspect.full_name,
                            game.suspect.short_name
                        );

                        self.ongoing = None;

                        HandlerResult::Response(reply)
                    } else {
                        HandlerResult::NoResponse
                    }
                }
            }
            _ => HandlerResult::Unhandled,
        }
    }
}

fn list_suspects(messages: &MessageDump) -> Vec<String> {
    messages
        .authors
        .iter()
        .enumerate()
        .map(|(idx, author)| {
            format!(
                "{}) {} под псевдонимом \"{}\"",
                idx + 1,
                author.full_name,
                author.short_name,
            )
        })
        .collect()
}

fn pick_random_suspect<'a>(
    messages: &'a MessageDump,
    rng: &mut SmallRng,
) -> (&'a messages::Author, Vec<&'a str>) {
    let enum_authors = messages.authors.iter().enumerate().collect::<Vec<_>>();
    let (author_idx, author) = enum_authors.choose(rng).unwrap();
    let messages_by_author = messages
        .texts
        .iter()
        .filter(|m| m.author_idx == *author_idx)
        .collect::<Vec<_>>();
    let sample_messages = messages_by_author
        .choose_multiple(rng, MESSAGES_SHOWN)
        .map(|m| m.text.as_ref())
        .collect::<Vec<_>>();

    (author, sample_messages)
}
