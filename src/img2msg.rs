use crate::{messages::MessageDump, JoeResult};
use rand::{rngs::SmallRng, SeedableRng};
use serenity::{model::prelude::*, prelude::*};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::Arc;

pub struct Img2msg {
    messages: Arc<MessageDump>,
    rng: SmallRng,
    classifier: UnixStream,
}

impl Img2msg {
    pub fn new(messages: Arc<MessageDump>) -> JoeResult<Self> {
        let classifier = UnixStream::connect("imclassif.sock")?;

        Ok(Self {
            messages,
            rng: SmallRng::from_entropy(),
            classifier,
        })
    }

    pub fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
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

                if let Some(t) = pick_text(&self.messages, &mut self.rng, &tiered_kw_stems) {
                    msg.channel_id.say(&ctx.http, t)?;
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
) -> Option<&'a str> {
    keyword_stems_by_tier
        .iter()
        .find_map(|stems| messages.random_message_with_any_stem(stems, rng))
        .map(|msg| msg.text.as_str())
}
