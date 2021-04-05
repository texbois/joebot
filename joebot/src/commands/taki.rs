use crate::{
    config::{Config, UserMatcher},
    messages::{Author, MessageDump},
    storage, JoeResult,
};
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use serenity::{model::prelude::*, prelude::*};
use std::collections::BTreeMap;
use std::fmt::Write;

mod picker;
use picker::SuspectPicker;

const MAX_TRIES: usize = 5;
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
const WIN_STREAK_MESSAGES: [&str; 1] = ["Пусть удача светит тебе ярче пустынного солнца, ковбой"];
const LOSE_MESSAGES: [&str; 4] = [
    "Казино не взломано.",
    "Игра закрыта, неудачники.",
    "Очень жаль, но вы проиграли... Жалкие псы!",
    "Удачи в другой раз, амигос.",
];

const KEY_BEST_STREAK: &str = "streaks";
const KEY_CURR_STREAK: &str = "currstreak";
const KEY_SCORE: &str = "scores";

const KEY_NUM_TRIES: &str = "numtries";
const KEY_NUM_WINS: &str = "numwins";

pub struct Taki<'a> {
    suspect_picker: SuspectPicker<'a>,
    suspect_matcher: &'a UserMatcher,
    storage: storage::ChatGameStorage,
    ongoing: Option<OngoingGame<'a>>,
    rng: SmallRng,
}

struct OngoingGame<'a> {
    suspect: &'a Author,
    answers: Vec<UserId>,
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
                    answers: Vec::with_capacity(MAX_TRIES),
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

                let mut user_cache: BTreeMap<u64, User> = BTreeMap::new();

                let scores = self.storage.fetch_sorted_set(KEY_SCORE)?;
                writeln!(&mut stats, "Самые именитые стрелки:")?;
                for (index, (uid, score)) in scores.into_iter().enumerate() {
                    if !user_cache.contains_key(&uid) {
                        user_cache.insert(uid, UserId(uid).to_user(ctx)?);
                    }
                    writeln!(
                        &mut stats,
                        "{}) {} — {}",
                        index + 1,
                        user_cache[&uid].name,
                        score
                    )?;
                }

                let streaks = self.storage.fetch_sorted_set(KEY_BEST_STREAK)?;
                writeln!(&mut stats, "\nСамые удачливые стрелки:")?;
                for (index, (uid, streak)) in streaks.into_iter().enumerate() {
                    if streak < 1 {
                        continue;
                    }
                    if !user_cache.contains_key(&uid) {
                        user_cache.insert(uid, UserId(uid).to_user(ctx)?);
                    }
                    writeln!(
                        &mut stats,
                        "{}) {} — {}",
                        index + 1,
                        user_cache[&uid].name,
                        streak
                    )?;
                }

                let tries = self.storage.fetch_sorted_set(KEY_NUM_TRIES)?;
                let wins = self.storage.fetch_sorted_set(KEY_NUM_WINS)?;

                let mut kdratios: Vec<(u64, u32)> = wins
                    .iter()
                    .filter_map(|&(uid, numt)| {
                        tries
                            .iter()
                            .find(|&&(tuid, _)| tuid == uid)
                            .map(|&(_, numw)| (uid, ((numt as f32 / numw as f32) * 100.0) as u32))
                    })
                    .collect();
                kdratios.sort_by(|(_, r1), (_, r2)| r2.cmp(&r1)); // desc

                writeln!(&mut stats, "\nСамые меткие стрелки:")?;
                for (index, (uid, kd)) in kdratios.into_iter().enumerate() {
                    if !user_cache.contains_key(&uid) {
                        user_cache.insert(uid, UserId(uid).to_user(ctx)?);
                    }
                    writeln!(
                        &mut stats,
                        "{}) {} — {}%",
                        index + 1,
                        user_cache[&uid].name,
                        kd
                    )?;
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

                let uid: u64 = msg.author.id.0;
                self.storage.incr_in_set(KEY_NUM_TRIES, uid, 1)?;

                game.answers.push(msg.author.id);

                if self.suspect_matcher.matches_short_name(&text, suspect_name) {
                    self.storage.incr_in_set(KEY_NUM_WINS, uid, 1)?;

                    let score: u32 = (MAX_TRIES + 1 - game.answers.len()) as u32;
                    self.storage.incr_in_set(KEY_SCORE, uid, score as i32)?;

                    for answer_uid in &game.answers {
                        if answer_uid.0 != uid {
                            self.storage.rem_from_set(KEY_CURR_STREAK, answer_uid.0)?;
                        }
                    }

                    // Guessed on the first try?
                    let curr_streak =
                        if game.answers.iter().filter(|&&a| a == msg.author.id).count() == 1 {
                            self.storage.incr_in_set(KEY_CURR_STREAK, uid, 1)?
                        } else {
                            self.storage.rem_from_set(KEY_CURR_STREAK, uid)?;
                            0
                        };

                    self.storage
                        .add_gt_to_set(KEY_BEST_STREAK, uid, curr_streak)?;
                    let best_streak = self.storage.get_in_set(KEY_BEST_STREAK, uid)?;

                    let (title, score_msg) = match curr_streak {
                        0 | 1 => {
                            let title = WIN_MESSAGES.choose(&mut self.rng).unwrap();
                            (title, format!("{} получает +{}", msg.author.name, score))
                        }
                        _ => {
                            let hits_word = if curr_streak % 100 > 4 && curr_streak % 100 < 20 {
                                "попаданий"
                            } else {
                                match curr_streak % 10 {
                                    1 => "попадание",
                                    2 | 3 | 4 => "попадания",
                                    _ => "попаданий",
                                }
                            };
                            let msg = if curr_streak < best_streak {
                                format!("{} пришпорил коня и понесся вперед! +{} и {} {} подряд в кармане", msg.author.name, score, curr_streak, hits_word)
                            } else {
                                format!("{} сегодня определенно в ударе! {} {} подряд, я чуть из седла не выпал, когда услыхал! Забирай свои +{} и мчись вперед", msg.author.name, curr_streak, hits_word, score)
                            };
                            let title = WIN_STREAK_MESSAGES.choose(&mut self.rng).unwrap();
                            (title, msg)
                        }
                    };

                    let resp = format!(
                        "Это был _{}_ под псевдонимом `{}`\n\n{}",
                        game.suspect.full_name, game.suspect.short_name, score_msg
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
                } else {
                    if game.answers.len() == MAX_TRIES {
                        let title = LOSE_MESSAGES.choose(&mut self.rng).unwrap();
                        let resp = format!(
                            "Это был _{}_ под псевдонимом `{}`",
                            game.suspect.full_name, game.suspect.short_name
                        );

                        // Your streaks end here, partners. Easy come, easy go...
                        for uid in &game.answers {
                            self.storage.rem_from_set(KEY_CURR_STREAK, uid.0)?;
                        }

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
