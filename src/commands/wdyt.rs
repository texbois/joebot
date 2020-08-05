use crate::{messages::MessageDump, JoeResult};
use rand::{rngs::SmallRng, SeedableRng};
use regex::Regex;
use rust_stemmers::Stemmer;
use serenity::{model::prelude::*, prelude::*};
use std::borrow::Cow;

pub struct Wdyt<'a> {
    messages: &'a MessageDump,
    trigger_regex: Regex,
    en_stemmer: Stemmer,
    ru_stemmer: Stemmer,
    rng: SmallRng,
}

impl<'a> Wdyt<'a> {
    pub fn new(messages: &'a MessageDump) -> JoeResult<Self> {
        let trigger_regex = Regex::new(r"(?i)(?:что (?:ты )?думаешь (?:об?|про|насчет)|как тебе|(?:тво[её]|ваше) мнение об?|как (?:ты )?относишься ко?)\s+(.+)").unwrap();
        let en_stemmer = Stemmer::create(rust_stemmers::Algorithm::English);
        let ru_stemmer = Stemmer::create(rust_stemmers::Algorithm::Russian);
        let rng = SmallRng::from_entropy();

        Ok(Self {
            messages,
            trigger_regex,
            en_stemmer,
            ru_stemmer,
            rng,
        })
    }

    pub fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
        if let Some(captures) = self.trigger_regex.captures(&msg.content) {
            let words = captures[1]
                .split(' ')
                .map(|w| w.to_lowercase())
                .collect::<Vec<_>>();
            let stems = words
                .iter()
                .map(|w| {
                    if w.is_ascii() {
                        self.en_stemmer.stem(&w)
                    } else {
                        self.ru_stemmer.stem(&w)
                    }
                })
                .collect::<Vec<_>>();

            let resp = match pick_text(&self.messages, &mut self.rng, &stems) {
                Some(t) => t,
                _ => r"¯\_(ツ)_/¯",
            };
            msg.channel_id.say(&ctx.http, resp)?;

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

fn pick_text<'a>(
    messages: &'a MessageDump,
    rng: &mut SmallRng,
    stems: &[Cow<str>],
) -> Option<&'a str> {
    messages
        .random_message_with_all_stems(stems, rng)
        .map(|msg| msg.text.as_str())
}
