use crate::JoeResult;
use serenity::{
    client::Context,
    model::channel::{Message, Reaction},
};

mod chain;
mod img2msg;
mod joker;
mod poll;
mod taki;
mod wdyt;
mod randtext;

pub use chain::Chain;
pub use img2msg::Img2msg;
pub use joker::Joker;
pub use poll::Poll;
pub use taki::Taki;
pub use wdyt::Wdyt;
pub use randtext::RandText;

pub trait Command {
    fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool>;

    fn handle_reaction(&mut self, _ctx: &Context, _rct: &Reaction) -> JoeResult<bool> {
        Ok(false)
    }
}

pub struct CommandDispatcher<'a> {
    commands: Vec<Box<dyn Command + 'a + Send + Sync>>,
}

impl<'a> CommandDispatcher<'a> {
    pub fn new(commands: Vec<Box<dyn Command + 'a + Send + Sync>>) -> Self {
        Self { commands }
    }

    pub fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
        for cmd in &mut self.commands {
            if cmd.handle_message(ctx, msg)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn handle_reaction(&mut self, ctx: &Context, rct: &Reaction) -> JoeResult<bool> {
        for cmd in &mut self.commands {
            if cmd.handle_reaction(ctx, rct)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
