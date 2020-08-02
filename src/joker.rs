use crate::{
    messages::{self, MessageDump},
    storage, JoeResult,
};
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use serenity::{model::prelude::*, prelude::*, http::AttachmentType};
use std::fmt::Write;
use std::sync::Arc;
use std::borrow::Cow;
use artano::Font;

pub struct Joker {
    messages: Arc<MessageDump>,
    font: Font<'static>
}

impl Joker {
    pub fn new(messages: Arc<MessageDump>) -> JoeResult<Self> {
        let font = Font::try_from_vec(std::fs::read("joker/font.ttf")?).unwrap();
        Ok(Self { messages, font })
    }

    pub fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
        if msg.content == "!joker" {
            let raw_img = std::fs::read("joker/1.jpg")?;
            let mut canvas = artano::Canvas::read_from_buffer(&raw_img)?;

            let top = artano::Annotation::top("joker");
            let bottom = artano::Annotation::bottom("bottom text");

            canvas.add_annotation(&top, &self.font, 1.0);
            canvas.add_annotation(&bottom, &self.font, 1.0);

            canvas.render();

            let mut res = Vec::new();
            canvas.save_jpg(&mut res)?;

            msg.channel_id.send_message(&ctx.http, |m| {
                m.add_file(AttachmentType::Bytes { data: Cow::from(res), filename: "joker.jpg".into() });
                m
            })?;

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
