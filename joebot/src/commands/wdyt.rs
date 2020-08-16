use crate::{messages::MessageDump, JoeResult};
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use regex::Regex;
use serenity::{model::prelude::*, prelude::*};

pub struct Wdyt<'a> {
    messages: &'a MessageDump,
    trigger_regex: Regex,
    rng: SmallRng,
}

impl<'a> Wdyt<'a> {
    pub fn new(messages: &'a MessageDump) -> JoeResult<Self> {
        let trigger_regex = Regex::new(r"(?i)(?:что (?:ты )?думаешь (?:об?|про|насчет)|как тебе|(?:тво[её]|ваше) мнение об?|как (?:ты )?относишься ко?)\s+(?P<prompt>.+)").unwrap();
        let rng = SmallRng::from_entropy();

        Ok(Self {
            messages,
            trigger_regex,
            rng,
        })
    }
}

impl<'a> super::Command for Wdyt<'a> {
    fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
        if let Some(captures) = self.trigger_regex.captures(&msg.content) {
            let prompt = &&captures["prompt"];
            let resp = self
                .messages
                .containing_all_words(prompt)
                .choose(&mut self.rng)
                .map(|msg| msg.text.as_str())
                .unwrap_or(r"¯\_(ツ)_/¯");

            msg.channel_id.say(&ctx.http, resp)?;

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
