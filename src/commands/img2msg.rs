use crate::{
    messages::{Author, MessageDump},
    JoeResult,
};
use rand::{rngs::SmallRng, SeedableRng};
use serenity::{model::prelude::*, prelude::*};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

pub struct Img2msg<'a> {
    messages: &'a MessageDump,
    rng: SmallRng,
    classifier: UnixStream,
}

impl<'a> Img2msg<'a> {
    pub fn new(messages: &'a MessageDump) -> JoeResult<Self> {
        let classifier = UnixStream::connect("imclassif.sock")?;

        Ok(Self {
            messages,
            rng: SmallRng::from_entropy(),
            classifier,
        })
    }
}

impl<'a> super::Command for Img2msg<'a> {
    fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
        match msg.attachments.first() {
            Some(a) if a.width.is_some() => {
                let data = a.download()?;

                self.classifier.write(&(data.len() as u32).to_be_bytes())?;
                self.classifier.write(&data)?;

                let mut result_size_bytes = [0u8; 4];
                self.classifier.read_exact(&mut result_size_bytes)?;
                let result_size = u32::from_be_bytes(result_size_bytes);

                let mut result_bytes = vec![0; result_size as usize];
                self.classifier.read_exact(&mut result_bytes)?;

                let result = std::str::from_utf8(&result_bytes)?;
                let tiered_kw_stems = result
                    .split(';')
                    .map(|ss| ss.split(',').collect::<Vec<_>>())
                    .collect::<Vec<_>>();

                println!("{:?}", tiered_kw_stems);

                if let Some((author, text)) =
                    pick_text(&self.messages, &mut self.rng, &tiered_kw_stems)
                {
                    let kw_stems = tiered_kw_stems
                        .iter()
                        .map(|ss| *ss.first().unwrap())
                        .collect::<Vec<_>>()
                        .join(", ");

                    msg.channel_id.send_message(&ctx.http, |m| {
                        m.embed(|e| {
                            e.color(crate::EMBED_COLOR);
                            e.description(text);
                            e.footer(|f| {
                                f.text(format!("— {} о {}", author.short_name, kw_stems));
                                f
                            });
                            e
                        });
                        m
                    })?;
                }

                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

fn pick_text<'a>(
    messages: &'a MessageDump,
    rng: &mut SmallRng,
    keyword_stems_by_tier: &[Vec<&str>],
) -> Option<(&'a Author, &'a str)> {
    keyword_stems_by_tier
        .iter()
        .find_map(|stems| messages.random_message_with_any_stem(stems, rng))
        .map(|msg| (&messages.authors[msg.author_idx], msg.text.as_str()))
}
