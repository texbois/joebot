use std::error::Error;
use std::fs::File;
use std::sync::Arc;

pub type JoeResult<T> = Result<T, Box<dyn Error>>;

mod messages;
mod storage;

use serenity::{model::prelude::*, prelude::*};

mod chain;
mod joker;
mod taki;

struct MessageHandlers {
    taki: taki::Taki,
    chain: chain::Chain,
    joker: joker::Joker,
}

struct Handler {
    bot_channel_id: ChannelId,
    message_handlers: Mutex<MessageHandlers>,
}

impl Handler {
    fn handle_message(&self, ctx: Context, msg: Message) -> JoeResult<()> {
        let mut handlers = self.message_handlers.lock();
        let taki_result = handlers.taki.handle_message(&ctx, &msg);
        if taki_result.map_err(|e| format!("Taki: {:?}", e))? {
            return Ok(());
        }
        let chain_result = handlers.chain.handle_message(&ctx, &msg);
        if chain_result.map_err(|e| format!("Chain: {:?}", e))? {
            return Ok(());
        }
        let joker_result = handlers.joker.handle_message(&ctx, &msg);
        if joker_result.map_err(|e| format!("Joker: {:?}", e))? {
            return Ok(());
        }
        if msg.content == "!ping" {
            msg.channel_id
                .say(&ctx.http, "Pong!")
                .map_err(|e| format!("Ping: {:?}", e))?;
        }
        Ok(())
    }
}

impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        println!("Connected as {}", ready.user.name);
    }

    fn message(&self, ctx: Context, msg: Message) {
        if msg.channel_id != self.bot_channel_id {
            return;
        }
        if let Err(e) = self.handle_message(ctx, msg) {
            eprintln!("{}", e)
        }
    }
}

fn main() {
    let bot_token = std::env::var("BOT_TOKEN")
        .expect("Provide a valid bot token via the BOT_TOKEN environment variable");
    let bot_channel_id: u64 = std::env::var("BOT_CHANNEL_ID")
        .ok()
        .and_then(|id| id.parse().ok())
        .expect("Provide the bot's channel id via the BOT_CHANNEL_ID environment variable");
    let redis = storage::Redis::new("redis://127.0.0.1/")
        .map_err(|e| format!("redis: {}", e))
        .unwrap();

    let message_handlers = Mutex::new(init_handlers(bot_channel_id, &redis));
    let handler = Handler {
        bot_channel_id: ChannelId(bot_channel_id),
        message_handlers,
    };
    let mut client = Client::new(&bot_token, handler).unwrap();

    if let Err(e) = client.start() {
        eprintln!("Client error: {:?}", e);
    }
}

fn init_handlers(channel_id: u64, redis: &storage::Redis) -> MessageHandlers {
    let messages = init_messages();
    let chain_data: joebot_markov_chain::MarkovChain =
        bincode::deserialize_from(File::open("chain.bin").unwrap()).unwrap();

    let taki = taki::Taki::new(messages.clone(), channel_id, redis);
    let chain = chain::Chain::new(chain_data);
    let joker = joker::Joker::new(messages.clone()).unwrap();

    MessageHandlers { taki, chain, joker }
}

fn init_messages() -> Arc<messages::MessageDump> {
    let msg_name_env = std::env::var("MSG_NAMES").unwrap_or_default();
    let msg_names: Vec<&str> = msg_name_env.split(',').map(|n| n.trim()).collect();

    let messages = messages::MessageDump::from_file("messages.html", &msg_names);
    let message_authors = messages
        .authors
        .iter()
        .map(|a| a.full_name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "{} messages from the following authors: {}\n",
        messages.texts.len(),
        message_authors
    );
    Arc::new(messages)
}
