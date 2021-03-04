use crate::{
    messages::{Author, MessageDump},
    JoeResult,
};
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use regex::Regex;
use serenity::{http::AttachmentType, model::prelude::*, prelude::*};
use std::borrow::Cow;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

mod template;

pub struct Joker<'a> {
    messages: &'a MessageDump,
    trigger_regex: Regex,
    rng: SmallRng,
    templates: Vec<template::Template>,
    random_text_generator: UnixStream,
}

impl<'a> Joker<'a> {
    pub fn new(messages: &'a MessageDump) -> JoeResult<Self> {
        let trigger_regex =
            Regex::new(r"(?i)(?:джокер)\s*(?P<len>[+]+)?(?:\s*про\s+(?:(?P<prompt_top>.+)\s+и\s+(?P<prompt_bottom>.+)|(?P<prompt>.+)))?").unwrap();
        let rng = SmallRng::from_entropy();
        let templates = template::load_jpg_templates("joker")?;
        let random_text_generator = UnixStream::connect("randtext.sock")?;

        Ok(Self {
            messages,
            trigger_regex,
            rng,
            templates,
            random_text_generator,
        })
    }
}

impl<'a> super::Command for Joker<'a> {
    fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
        if let Some(captures) = self.trigger_regex.captures(&msg.content) {
            let top_prompt = captures
                .name("prompt")
                .or_else(|| captures.name("prompt_top"))
                .map(|m| m.as_str());
            let bottom_prompt = captures.name("prompt_bottom").map(|m| m.as_str());

            if top_prompt.is_none() && bottom_prompt.is_none() {
                let (top_text, bottom_text) =
                    generate_top_bottom_text(&mut self.random_text_generator)?;

                self.send_image(
                    msg.channel_id,
                    ctx,
                    &top_text,
                    &bottom_text,
                    "This meme was made by quest snickers and joe",
                )?;
            } else {
                let min_words = captures
                    .name("len")
                    .map(|pluses| pluses.as_str().len())
                    .unwrap_or(0);

                let picks = pick_text(&self.messages, &mut self.rng, min_words, top_prompt)
                    .and_then(|top_pick| {
                        pick_text(&self.messages, &mut self.rng, min_words, bottom_prompt)
                            .map(|bottom_pick| (top_pick, bottom_pick))
                    });

                if let Some(((top_author, top_text), (bottom_author, bottom_text))) = picks {
                    self.send_image(
                        msg.channel_id,
                        ctx,
                        top_text,
                        bottom_text,
                        &format!(
                            "This meme was made by {}, {} and joe",
                            top_author.short_name, bottom_author.short_name
                        ),
                    )?;
                } else {
                    msg.channel_id
                        .say(&ctx.http, "Наивное общество о таком еще не слыхало...")?;
                }
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl<'a> Joker<'a> {
    fn send_image(
        &mut self,
        channel_id: ChannelId,
        ctx: &Context,
        top_text: &str,
        bottom_text: &str,
        credits: &str,
    ) -> JoeResult<()> {
        let time_started = std::time::Instant::now();

        let template = self.templates.choose(&mut self.rng).unwrap();
        let img = template.render(top_text, bottom_text, "joker/font.ttf")?;

        let time_render = std::time::Instant::now();

        channel_id.send_files(
            &ctx.http,
            vec![AttachmentType::Bytes {
                data: Cow::from(img),
                filename: "joker.jpg".into(),
            }],
            |m| {
                m.embed(|e| {
                    e.color(crate::EMBED_COLOR);
                    e.attachment("joker.jpg");
                    e.footer(|f| f.text(credits));
                    e
                });
                m
            },
        )?;

        let time_message = std::time::Instant::now();
        println!(
            "Joker: image render: {}ms, network: {}ms",
            (time_render - time_started).as_millis(),
            (time_message - time_render).as_millis()
        );

        Ok(())
    }
}

fn generate_top_bottom_text(generator: &mut UnixStream) -> JoeResult<(String, String)> {
    const MAX_TEXT_LEN: u32 = 140;
    generator.write_all(&MAX_TEXT_LEN.to_be_bytes())?;

    let mut result_size_bytes = [0u8; 4];
    generator.read_exact(&mut result_size_bytes)?;
    let result_size = u32::from_be_bytes(result_size_bytes);

    let mut result_bytes = vec![0; result_size as usize];
    generator.read_exact(&mut result_bytes)?;

    let result = std::str::from_utf8(&result_bytes)?;

    let words = result.split_ascii_whitespace().collect::<Vec<_>>();

    if words.len() < 4 {
        Ok((result.into(), String::new()))
    } else {
        let (top_words, bottom_words) = words.split_at(words.len() / 2);
        Ok((top_words.join(" "), bottom_words.join(" ")))
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
