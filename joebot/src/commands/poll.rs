use crate::JoeResult;
use regex::Regex;
use serenity::{model::prelude::*, prelude::*, utils::MessageBuilder};
use std::fmt::Write;

pub struct Poll {
    item_re: Regex,
}

impl Poll {
    pub fn new() -> Self {
        Self {
            item_re: Regex::new(r#""([^"]+)""#).unwrap(),
        }
    }
}

impl super::Command for Poll {
    fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
        if !msg.content.starts_with("!poll") {
            return Ok(false);
        }

        let mut items = self.item_re.captures_iter(&msg.content).peekable();
        match items.next() {
            // topic + at least one option provided
            Some(topic_cap) if items.peek().is_some() => {
                let mut body = String::new();
                let mut reactions = Vec::new();
                for (i, item_cap) in items.enumerate() {
                    let reaction = std::char::from_u32(0x1f1e6 as u32 + i as u32)
                        .unwrap()
                        .to_string();
                    writeln!(&mut body, "{} {}", reaction, &item_cap[1])?;
                    reactions.push(ReactionType::Unicode(reaction));
                }
                msg.channel_id.send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.title(&topic_cap[1]);
                        e.description(body);
                        e
                    });
                    m.reactions(reactions);
                    m
                })?;
            }
            _ => {
                let help = MessageBuilder::new()
                    .push_mono(r#"!poll "OUPOC" "AGA" "NE""#)
                    .build();
                msg.channel_id.say(&ctx.http, help)?;
            }
        }

        Ok(true)
    }
}
