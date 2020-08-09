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
        let trigger_regex = Regex::new("(?i)(?:джокер)[а-я ]*([+]+)?").unwrap();
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
            let min_words = captures
                .get(1)
                .map(|pluses| pluses.as_str().len())
                .unwrap_or(0);

            let time_started = std::time::Instant::now();
            if let Some(((top_author, top_text), (bottom_author, bottom_text))) =
                pick_top_bottom(&self.messages, &mut self.rng, min_words)
            {
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
                let resp = std::iter::repeat("society")
                    .take(min_words)
                    .collect::<Vec<_>>()
                    .join(" ");
                msg.channel_id.say(&ctx.http, resp)?;
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

fn pick_top_bottom<'a>(
    messages: &'a MessageDump,
    rng: &mut SmallRng,
    min_words: usize,
) -> Option<((&'a Author, &'a str), (&'a Author, &'a str))> {
    let max_len = std::cmp::max(200, 50 + 8 * min_words);

    let texts = messages
        .texts
        .iter()
        .filter(|m| {
            (min_words == 0 || m.text.matches(' ').count() >= (min_words - 1))
                && m.text.chars().count() < max_len
        })
        .collect::<Vec<_>>();

    let top = texts
        .choose(rng)
        .map(|msg| (&messages.authors[msg.author_idx], msg.text.as_str()))?;
    let bottom = texts
        .choose(rng)
        .map(|msg| (&messages.authors[msg.author_idx], msg.text.as_str()))?;

    Some((top, bottom))
}