use serenity::{model::prelude::*, prelude::*};
use std::fmt::Write;

use crate::{
    messages::{self, MessageDump},
    storage, JoeResult,
};
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};

const INIT_SCORE: i32 = 5;
const MESSAGES_SHOWN: usize = 3;
const START_MESSAGES: [(&str, &str); 3] = [
    ("Один мудрец сказал:", "Кто же это был?"),
    (
        "Последний раз подозреваемого видели в местном баре, где он произнес:",
        "Найдите мне этого пса!",
    ),
    ("Дружок-пирожок оставил вам послание:", "Узнали?"),
];
const WIN_MESSAGES: [&str; 3] = [
    "Хорошая работа, дружище.",
    "А ты неплох, приятель.",
    "Дело сделано, джентельмены.",
];
const LOSE_MESSAGES: [&str; 4] = [
    "Казино не взломано.",
    "Игра закрыта, неудачники.",
    "Очень жаль, но вы проиграли.",
    "Удачи в другой раз, амигос.",
];

use std::sync::Arc;

pub struct Taki {
    messages: Arc<MessageDump>,
    storage: storage::ChatGameStorage,
    ongoing: Option<OngoingGame>,
    rng: SmallRng,
}

struct OngoingGame {
    suspect: messages::Author,
    score: i32,
}

impl Taki {
    pub fn new(messages: Arc<MessageDump>, chat_id: u64, redis: &storage::Redis) -> Self {
        Self {
            messages,
            storage: redis.get_game_storage("taki", chat_id),
            ongoing: None,
            rng: SmallRng::from_entropy(),
        }
    }

    pub fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
        match (msg.content.as_str(), &mut self.ongoing) {
            ("!takistart", None) => {
                let (suspect, messages) = pick_random_suspect(&self.messages, &mut self.rng);

                self.ongoing = Some(OngoingGame {
                    suspect: suspect.clone(),
                    score: INIT_SCORE,
                });
                let (start_prefix, start_suffix) = START_MESSAGES.choose(&mut self.rng).unwrap();

                let resp = format!(
                    "{}\n\n* {}\n\n{}",
                    start_prefix,
                    messages.join("\n* "),
                    start_suffix
                );

                msg.channel_id.say(&ctx.http, resp)?;
                Ok(true)
            }
            ("!takistats", _) => {
                let mut stats = String::new();

                let scores = self.storage.fetch_sorted_set("scores")?;

                for (index, (uid, score)) in scores.into_iter().enumerate() {
                    let user = UserId(uid).to_user(ctx)?;
                    write!(&mut stats, "{}) {} -- {}\n", index + 1, user.name, score)?;
                }

                msg.channel_id
                    .say(&ctx.http, format!("Статы:\n{}", stats))?;
                Ok(true)
            }
            ("!takisuspects", _) => {
                let suspects = list_suspects(&self.messages).join("\n");
                let resp = format!("Подозреваемые:\n{}", suspects);

                msg.channel_id.say(&ctx.http, resp)?;
                Ok(true)
            }
            (_, Some(ref mut game)) => {
                let text_lower = msg.content.to_lowercase();

                if text_lower == game.suspect.short_name.to_lowercase()
                    || text_lower == game.suspect.full_name.to_lowercase()
                {
                    let reply = format!(
                        "{}\n{} +{}",
                        WIN_MESSAGES.choose(&mut self.rng).unwrap(),
                        msg.author.name,
                        game.score
                    );

                    self.storage
                        .incr_in_set("scores", msg.author.id.0, game.score)?;
                    self.ongoing = None;

                    msg.channel_id.say(&ctx.http, reply)?;
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

                        msg.channel_id.say(&ctx.http, reply)?;
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
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
