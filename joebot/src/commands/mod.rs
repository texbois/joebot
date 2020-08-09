use crate::JoeResult;
use serenity::{client::Context, model::channel::Message};

mod chain;
mod img2msg;
mod joker;
mod poll;
mod taki;
mod wdyt;

pub use chain::Chain;
pub use img2msg::Img2msg;
pub use joker::Joker;
pub use poll::Poll;
pub use taki::Taki;
pub use wdyt::Wdyt;

pub trait Command {
    fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool>;
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
            let result = cmd.handle_message(ctx, msg)?;
            if result {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
