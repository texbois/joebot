use std::error::Error;
use std::fs::File;
use std::sync::Arc;

pub type JoeResult<T> = Result<T, Box<dyn Error>>;

mod messages;
mod storage;

use serenity::{model::prelude::*, prelude::*};

mod chain;
mod taki;

struct Handler {
    bot_channel_id: ChannelId,
    taki: Mutex<taki::Taki>,
    chain: Mutex<chain::Chain>,
}

impl Handler {
    fn handle_message(&self, ctx: Context, msg: Message) -> JoeResult<()> {
        let taki_result = self.taki.lock().handle_message(&ctx, &msg);
        if taki_result.map_err(|e| format!("Taki: {:?}", e))? {
            return Ok(());
        }
        let chain_result = self.chain.lock().handle_message(&ctx, &msg);
        if chain_result.map_err(|e| format!("Chain: {:?}", e))? {
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

    let taki = Mutex::new(init_taki(bot_channel_id, &redis));
    let chain = Mutex::new(init_chain());

    let handler = Handler {
        bot_channel_id: ChannelId(bot_channel_id),
        taki,
        chain,
    };
    let mut client = Client::new(&bot_token, handler).unwrap();

    if let Err(e) = client.start() {
        eprintln!("Client error: {:?}", e);
    }
}

fn init_taki(channel_id: u64, redis: &storage::Redis) -> taki::Taki {
    let taki_names_env = std::env::var("TAKI_NAMES").unwrap_or_default();
    let taki_names: Vec<&str> = taki_names_env.split(',').map(|n| n.trim()).collect();

    let messages = Arc::new(messages::MessageDump::from_file(
        "messages.html",
        &taki_names,
    ));
    let message_authors = messages
        .authors
        .iter()
        .map(|a| a.full_name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "Taki: {} messages from the following authors: {}\n",
        messages.texts.len(),
        message_authors
    );
    taki::Taki::new(messages, channel_id, redis)
}

fn init_chain() -> chain::Chain {
    let data: joebot_markov_chain::MarkovChain =
        bincode::deserialize_from(File::open("chain.bin").unwrap()).unwrap();

    chain::Chain::new(data)
}
