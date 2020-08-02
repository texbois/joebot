use crate::{messages::MessageDump, JoeResult};
use artano::Font;
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use regex::Regex;
use serenity::{http::AttachmentType, model::prelude::*, prelude::*};
use std::borrow::Cow;
use std::sync::Arc;

pub struct Joker {
    messages: Arc<MessageDump>,
    font: Font<'static>,
    mention_regex: Regex,
    rng: SmallRng,
    source_images: Vec<Vec<u8>>,
}

impl Joker {
    pub fn new(messages: Arc<MessageDump>) -> JoeResult<Self> {
        let font = Font::try_from_vec(std::fs::read("joker/font.ttf")?).unwrap();
        let mention_regex = Regex::new("(?i)(?:джокер)[а-я ]*([+]+)?").unwrap();
        let rng = SmallRng::from_entropy();

        let mut source_images = Vec::new();
        for entry in std::fs::read_dir("joker")? {
            let path = entry?.path();
            if path.extension().map(|s| s == "jpg").unwrap_or(false) {
                source_images.push(std::fs::read(path)?);
            }
        }

        Ok(Self {
            messages,
            font,
            mention_regex,
            rng,
            source_images,
        })
    }

    pub fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
        if let Some(captures) = self.mention_regex.captures(&msg.content) {
            let min_words = captures
                .get(1)
                .map(|pluses| pluses.as_str().len())
                .unwrap_or(0);

            let time_started = std::time::Instant::now();
            if let Some((top, bottom)) = pick_top_bottom(&self.messages, &mut self.rng, min_words) {
                let time_texts = std::time::Instant::now();

                let source_img = self.source_images.choose(&mut self.rng).unwrap();
                let img = make_img(source_img, top, bottom, &self.font)?;

                let time_render = std::time::Instant::now();

                msg.channel_id.send_message(&ctx.http, |m| {
                    m.add_file(AttachmentType::Bytes {
                        data: Cow::from(img),
                        filename: "joker.jpg".into(),
                    });
                    m
                })?;

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
) -> Option<(&'a str, &'a str)> {
    let max_len = std::cmp::max(150, 50 + 8 * min_words);

    let texts = messages
        .texts
        .iter()
        .filter(|m| {
            (min_words == 0 || m.text.matches(' ').count() >= (min_words - 1))
                && m.text.chars().count() < max_len
        })
        .collect::<Vec<_>>();

    let top = texts.choose(rng)?;
    let bottom = texts.choose(rng)?;

    Some((&top.text, &bottom.text))
}

fn make_img(source_img: &[u8], top: &str, bottom: &str, font: &Font) -> JoeResult<Vec<u8>> {
    let mut canvas = artano::Canvas::read_from_buffer(source_img)?;

    let top = artano::Annotation::top(top);
    let bottom = artano::Annotation::bottom(bottom);

    canvas.add_annotation(&top, font, 1.0);
    canvas.add_annotation(&bottom, font, 1.0);
    canvas.render();

    let mut res = Vec::new();
    canvas.save_jpg(&mut res)?;
    Ok(res)
}
