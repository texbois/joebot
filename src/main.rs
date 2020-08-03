use std::cell::RefCell;
use std::error::Error;
use std::fs::File;
use std::sync::Arc;

pub type JoeResult<T> = Result<T, Box<dyn Error>>;

mod messages;
mod storage;
mod utils;

use serenity::{model::prelude::*, prelude::*, utils::MessageBuilder};

mod chain;
mod img2msg;
mod joker;
mod taki;
mod wdyt;

struct MessageHandlers {
    taki: taki::Taki,
    chain: chain::Chain,
    joker: joker::Joker,
    wdyt: wdyt::Wdyt,
    img2msg: img2msg::Img2msg,
}

struct Handler {
    bot_user: Mutex<RefCell<Option<CurrentUser>>>,
    bot_channel_id: ChannelId,
    message_handlers: Mutex<MessageHandlers>,
}

impl Handler {
    fn handle_message(&self, ctx: Context, msg: Message) -> JoeResult<()> {
        if let Some(ref user) = *self.bot_user.lock().borrow() {
            if user.id == msg.author.id {
                return Ok(());
            }
        }

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
        let wdyt_result = handlers.wdyt.handle_message(&ctx, &msg);
        if wdyt_result.map_err(|e| format!("Wdyt: {:?}", e))? {
            return Ok(());
        }
        let img2msg_result = handlers.img2msg.handle_message(&ctx, &msg);
        if img2msg_result.map_err(|e| format!("Img2msg: {:?}", e))? {
            return Ok(());
        }
        if msg.content.starts_with('!') {
            let help = MessageBuilder::new()
                .push_mono("!takistart")
                .push_line(" — сыграем в таки")
                .push_mono("!takisuspects")
                .push_line(" — бросим взгляд на плакаты о розыске")
                .push_mono("!takistats")
                .push_line(" — поднимем бокал крепкого виски за самых метких стрелков")
                .push_mono("!mashup")
                .push_line(" — узнаем от бармена последние слухи")
                .push_mono("!mashupmore")
                .push_line(" — посплетничаем еще")
                .push_mono("!mashupstars")
                .push_line(" — поприветствуем жителей городка")
                .push_line("")
                .push_underline_line("поговорим с джо:")
                .push_mono_line("что думаешь об итмо и бонче")
                .push_mono_line("джокер++")
                .push_line("")
                .push_underline_line("займемся делом:")
                .push_italic_line("покажи джо фотокарточку, о которой хочешь узнать побольше")
                .build();
            msg.channel_id
                .say(&ctx.http, &help)
                .map_err(|e| format!("Help: {:?}", e))?;
        }
        Ok(())
    }
}

impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        println!("Connected as {}", ready.user.name);
        self.bot_user.lock().replace(Some(ready.user));
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
        bot_user: Mutex::new(RefCell::new(None)),
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
    let wdyt = wdyt::Wdyt::new(messages.clone()).unwrap();
    let img2msg = img2msg::Img2msg::new(messages.clone()).unwrap();

    MessageHandlers {
        taki,
        chain,
        joker,
        wdyt,
        img2msg,
    }
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
