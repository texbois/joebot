use crate::{messages::MessageDump, JoeResult};
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use regex::Regex;
use rust_stemmers::Stemmer;
use serenity::{model::prelude::*, prelude::*};
use std::borrow::Cow;
use std::sync::Arc;

pub struct Wdyt {
    messages: Arc<MessageDump>,
    trigger_regex: Regex,
    stemmer: Stemmer,
    rng: SmallRng,
}

impl Wdyt {
    pub fn new(messages: Arc<MessageDump>) -> JoeResult<Self> {
        let trigger_regex = Regex::new(r"(?i)(?:что (?:ты )?думаешь (?:об?|про|насчет)|как тебе|(?:тво[её]|ваше) мнение об?|как (?:ты )?относишься ко?)\s+(.+)").unwrap();
        let stemmer = Stemmer::create(rust_stemmers::Algorithm::Russian);
        let rng = SmallRng::from_entropy();

        Ok(Self {
            messages,
            trigger_regex,
            stemmer,
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
                .map(|w| self.stemmer.stem(&w))
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
    let texts = messages
        .texts
        .iter()
        .filter(|m| {
            if m.text.chars().count() >= 2000 {
                /* Exceeds the limit set by Discord */
                return false;
            }
            let words = m.text.split(' ').collect::<Vec<_>>();
            stems
                .iter()
                .all(|s| words.iter().any(|w| w.starts_with(s.as_ref())))
        })
        .collect::<Vec<_>>();

    texts.choose(rng).map(|t| t.text.as_str())
}
