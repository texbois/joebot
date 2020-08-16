use crate::{
    messages::{Author, MessageDump},
    JoeResult,
};
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use regex::Regex;
use serenity::{http::AttachmentType, model::prelude::*, prelude::*};
use std::borrow::Cow;

mod template;

pub struct Joker<'a> {
    messages: &'a MessageDump,
    trigger_regex: Regex,
    rng: SmallRng,
    templates: Vec<template::Template>,
}

impl<'a> Joker<'a> {
    pub fn new(messages: &'a MessageDump) -> JoeResult<Self> {
        let trigger_regex =
            Regex::new(r"(?i)(?:джокер)\s*(?P<len>[+]+)?(?:\s*про\s+(?:(?P<prompt_top>.+)\s+и\s+(?P<prompt_bottom>.+)|(?P<prompt>.+)))?").unwrap();
        let rng = SmallRng::from_entropy();
        let templates = template::load_jpg_templates("joker")?;

        Ok(Self {
            messages,
            trigger_regex,
            rng,
            templates,
        })
    }
}

impl<'a> super::Command for Joker<'a> {
    fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
        if let Some(captures) = self.trigger_regex.captures(&msg.content) {
            let time_started = std::time::Instant::now();

            let min_words = captures
                .name("len")
                .map(|pluses| pluses.as_str().len())
                .unwrap_or(0);

            let top_prompt = captures
                .name("prompt")
                .or_else(|| captures.name("prompt_top"))
                .map(|m| m.as_str());
            let bottom_prompt = captures.name("prompt_bottom").map(|m| m.as_str());

            let picks = pick_text(&self.messages, &mut self.rng, min_words, top_prompt).and_then(
                |top_pick| {
                    pick_text(&self.messages, &mut self.rng, min_words, bottom_prompt)
                        .map(|bottom_pick| (top_pick, bottom_pick))
                },
            );
            if let Some(((top_author, top_text), (bottom_author, bottom_text))) = picks {
                let time_texts = std::time::Instant::now();

                let template = self.templates.choose(&mut self.rng).unwrap();
                let img = template.render(top_text, bottom_text, "joker/font.ttf")?;

                let time_render = std::time::Instant::now();

                msg.channel_id.send_files(
                    &ctx.http,
                    vec![AttachmentType::Bytes {
                        data: Cow::from(img),
                        filename: "joker.jpg".into(),
                    }],
                    |m| {
                        m.embed(|e| {
                            e.color(crate::EMBED_COLOR);
                            e.attachment("joker.jpg");
                            e.footer(|f| {
                                f.text(format!(
                                    "This meme was made by {}, {} and joe",
                                    top_author.short_name, bottom_author.short_name
                                ));
                                f
                            });
                            e
                        });
                        m
                    },
                )?;

                let time_message = std::time::Instant::now();
                println!(
                    "Joker: total {}ms, text pick: {}ms, image render: {}ms, network: {}ms",
                    (time_message - time_started).as_millis(),
                    (time_texts - time_started).as_millis(),
                    (time_render - time_texts).as_millis(),
                    (time_message - time_render).as_millis()
                );
            } else {
                msg.channel_id.say(&ctx.http, "Наивное общество о таком еще не слыхало...")?;
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

fn pick_text<'a>(
    messages: &'a MessageDump,
    rng: &mut SmallRng,
    min_words: usize,
    str_prompt: Option<&'a str>,
) -> Option<(&'a Author, &'a str)> {
    let max_len = std::cmp::max(200, 50 + 8 * min_words);
    let len_filter = |m: &&crate::messages::Message| {
        if min_words != 0 {
            if m.text.matches(' ').count() < (min_words - 1) {
                return false;
            }
        }
        if m.text.chars().count() >= max_len {
            return false;
        }
        true
    };

    let potential_picks = if let Some(ref p) = str_prompt {
        messages
            .containing_any_words(p)
            .into_iter()
            .filter(len_filter)
            .collect::<Vec<_>>()
    } else {
        messages.texts.iter().filter(len_filter).collect::<Vec<_>>()
    };

    potential_picks
        .choose(rng)
        .map(|msg| (&messages.authors[msg.author_idx], msg.text.as_str()))
}
