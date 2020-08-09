use crate::{
    config::{Config, UserMatcher},
    messages::{Author, MessageDump},
    storage, JoeResult,
};
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use serenity::{model::prelude::*, prelude::*};
use std::fmt::Write;

mod picker;
use picker::SuspectPicker;

const INIT_SCORE: i32 = 5;
const MESSAGES_SHOWN: usize = 3;
const START_MESSAGES: [(&str, &str); 3] = [
    ("Один мудрец сказал:", "Кто же это был?"),
    (
        "Последний раз подозреваемого видели в соседнем баре, где он произнес:",
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
    "Очень жаль, но вы проиграли... Жалкие псы!",
    "Удачи в другой раз, амигос.",
];

pub struct Taki<'a> {
    suspect_picker: SuspectPicker<'a>,
    suspect_matcher: &'a UserMatcher,
    storage: storage::ChatGameStorage,
    ongoing: Option<OngoingGame<'a>>,
    rng: SmallRng,
}

struct OngoingGame<'a> {
    suspect: &'a Author,
    score: i32,
}

impl<'a> Taki<'a> {
    pub fn new(messages: &'a MessageDump, conf: &'a Config, redis: &storage::Redis) -> Self {
        let suspect_picker = SuspectPicker::new(messages, &conf.user_penalties);

        Self {
            suspect_picker,
            suspect_matcher: &conf.user_matcher,
            storage: redis.get_game_storage("taki", conf.channel_id),
            ongoing: None,
            rng: SmallRng::from_entropy(),
        }
    }
}

impl<'a> super::Command for Taki<'a> {
    fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
        match (msg.content.as_str(), &mut self.ongoing) {
            ("!takistart", None) => {
                let (suspect, messages) = self
                    .suspect_picker
                    .random_suspect(&mut self.rng, MESSAGES_SHOWN);

                self.ongoing = Some(OngoingGame {
                    suspect,
                    score: INIT_SCORE,
                });
                let (start_prefix, start_suffix) = START_MESSAGES.choose(&mut self.rng).unwrap();

                let resp = format!("* {}\n\n{}", messages.join("\n* "), start_suffix);

                msg.channel_id.send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.color(crate::EMBED_COLOR);
                        e.title(start_prefix);
                        e.description(resp);
                        e
                    });
                    m
                })?;

                Ok(true)
            }
            ("!takistats", _) => {
                let mut stats = String::new();

                let scores = self.storage.fetch_sorted_set("scores")?;

                for (index, (uid, score)) in scores.into_iter().enumerate() {
                    let user = UserId(uid).to_user(ctx)?;
                    writeln!(&mut stats, "{}) {} — {}", index + 1, user.name, score)?;
                }

                msg.channel_id.send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.color(crate::EMBED_COLOR);
                        e.title("Мастера Таки");
                        e.description(stats);
                        e
                    });
                    m
                })?;

                Ok(true)
            }
            ("!takisuspects", _) => {
                let suspects = self
                    .suspect_picker
                    .list_suspects()
                    .enumerate()
                    .map(|(idx, author)| {
                        format!(
                            "{}) _{}_ под псевдонимом `{}`",
                            idx + 1,
                            author.full_name,
                            author.short_name,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                msg.channel_id.send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.color(crate::EMBED_COLOR);
                        e.title("Подозреваемые");
                        e.description(suspects);
                        e
                    });
                    m
                })?;

                Ok(true)
            }
            (_, Some(ref mut game)) => {
                let text = msg.content.to_lowercase();
                let suspect_name = &game.suspect.short_name;

                if self.suspect_matcher.matches_short_name(&text, suspect_name) {
                    let title = WIN_MESSAGES.choose(&mut self.rng).unwrap();
                    let resp = format!(
                        "Это был _{}_ под псевдонимом `{}`\n\n{} получает +{}",
                        game.suspect.full_name,
                        game.suspect.short_name,
                        msg.author.name,
                        game.score
                    );

                    self.storage
                        .incr_in_set("scores", msg.author.id.0, game.score)?;
                    self.ongoing = None;

                    msg.channel_id.send_message(&ctx.http, |m| {
                        m.embed(|e| {
                            e.color(crate::EMBED_COLOR);
                            e.title(title);
                            e.description(resp);
                            e
                        });
                        m
                    })?;
                } else {
                    game.score -= 1;

                    if game.score == 0 {
                        let title = LOSE_MESSAGES.choose(&mut self.rng).unwrap();
                        let resp = format!(
                            "Это был _{}_ под псевдонимом `{}`",
                            game.suspect.full_name, game.suspect.short_name
                        );

                        self.ongoing = None;

                        msg.channel_id.send_message(&ctx.http, |m| {
                            m.embed(|e| {
                                e.color(crate::EMBED_COLOR);
                                e.title(title);
                                e.description(resp);
                                e
                            });
                            m
                        })?;
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
