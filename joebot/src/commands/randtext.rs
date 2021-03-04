use crate::JoeResult;
use serenity::{model::prelude::*, prelude::*};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

const MAX_TEXT_LEN: u32 = 140;

pub struct RandText {
    generator: UnixStream,
    prev_message_id: Option<MessageId>,
}

impl RandText {
    pub fn new() -> JoeResult<Self> {
        let generator = UnixStream::connect("randtext.sock")?;

        Ok(Self {
            generator,
            prev_message_id: None,
        })
    }
}

impl super::Command for RandText {
    fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
        if msg.content == "!random" {
            self.send_random_text(ctx, msg.channel_id)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn handle_reaction(&mut self, ctx: &Context, rct: &Reaction) -> JoeResult<bool> {
        match &rct.emoji {
            ReactionType::Unicode(e)
                if e == "🔁" && Some(rct.message_id) == self.prev_message_id =>
            {
                self.send_random_text(ctx, rct.channel_id)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

impl RandText {
    fn send_random_text(&mut self, ctx: &Context, channel_id: ChannelId) -> JoeResult<()> {
        self.generator.write_all(&MAX_TEXT_LEN.to_be_bytes())?;

        let mut result_size_bytes = [0u8; 4];
        self.generator.read_exact(&mut result_size_bytes)?;
        let result_size = u32::from_be_bytes(result_size_bytes);

        let mut result_bytes = vec![0; result_size as usize];
        self.generator.read_exact(&mut result_bytes)?;

        let result = std::str::from_utf8(&result_bytes)?;

        let m = channel_id.send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.color(crate::EMBED_COLOR);
                e.description(result);
                e
            });
            m.reactions(vec!['🔁']);
            m
        })?;
        self.prev_message_id = Some(m.id);
        Ok(())
    }
}
