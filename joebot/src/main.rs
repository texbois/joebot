use lazy_static::lazy_static;
use serenity::{builder::CreateMessage, model::prelude::*, prelude::*, utils::Color};
use std::cell::RefCell;
use std::collections::HashSet;
use std::error::Error;
use std::fs::File;

pub type JoeResult<T> = Result<T, Box<dyn Error>>;

pub const EMBED_COLOR: Color = Color::new(0x7a4c50);

mod commands;
mod config;
mod messages;
mod storage;
mod utils;

lazy_static! {
    static ref CONFIG: config::Config = {
        let conf_str = std::fs::read_to_string("config.json").expect("Cannot read config.json");
        serde_json::from_str(&conf_str).unwrap()
    };
    static ref MESSAGE_DUMP: messages::MessageDump = {
        let msg_names: HashSet<&str> = CONFIG.user_matcher.short_names();
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
        messages
    };
}

struct Handler<'a> {
    bot_user: Mutex<RefCell<Option<CurrentUser>>>,
    bot_channel_id: ChannelId,
    dispatcher: Mutex<commands::CommandDispatcher<'a>>,
}

impl<'a> Handler<'a> {
    fn handle_message(&self, ctx: Context, msg: Message) -> JoeResult<()> {
        if let Some(ref user) = *self.bot_user.lock().borrow() {
            if user.id == msg.author.id {
                return Ok(());
            }
        }
        if self.dispatcher.lock().handle_message(&ctx, &msg)? {
            return Ok(());
        }
        if msg.content.starts_with('!') {
            msg.channel_id
                .send_message(&ctx.http, bot_help)
                .map_err(|e| format!("Help: {:?}", e))?;
        }
        Ok(())
    }

    fn handle_reaction(&self, ctx: Context, rct: Reaction) -> JoeResult<()> {
        if let Some(ref user) = *self.bot_user.lock().borrow() {
            if user.id == rct.user_id {
                return Ok(());
            }
        }
        self.dispatcher.lock().handle_reaction(&ctx, &rct)?;
        Ok(())
    }
}

impl<'a> EventHandler for Handler<'a> {
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

    fn reaction_add(&self, ctx: Context, rct: Reaction) {
        if rct.channel_id != self.bot_channel_id {
            return;
        }
        if let Err(e) = self.handle_reaction(ctx, rct) {
            eprintln!("{}", e)
        }
    }
}

fn main() {
    let bot_token = std::env::var("BOT_TOKEN")
        .expect("Provide a valid bot token via the BOT_TOKEN environment variable");

    let redis = storage::Redis::new("redis://127.0.0.1/")
        .map_err(|e| format!("redis: {}", e))
        .unwrap();

    println!("* Starting command handlers");
    let dispatcher = Mutex::new(init_dispatcher(&CONFIG, &redis));
    let handler = Handler {
        bot_user: Mutex::new(RefCell::new(None)),
        bot_channel_id: ChannelId(CONFIG.channel_id),
        dispatcher,
    };

    println!("* Connecting to Discord");
    let mut client = Client::new(&bot_token, handler).unwrap();

    if let Err(e) = client.start() {
        eprintln!("Client error: {:?}", e);
    }
}

fn init_dispatcher<'a>(
    conf: &'a config::Config,
    redis: &storage::Redis,
) -> commands::CommandDispatcher<'a> {
    let chain_data: joebot_markov_chain::MarkovChain =
        bincode::deserialize_from(File::open("chain.bin").unwrap()).unwrap();

    let taki = commands::Taki::new(&MESSAGE_DUMP, &conf, redis);
    let chain = commands::Chain::new(chain_data);
    let poll = commands::Poll::new();
    let wdyt = commands::Wdyt::new(&MESSAGE_DUMP).unwrap();
    let joker = commands::Joker::new(&MESSAGE_DUMP).unwrap();
    let img2msg = commands::Img2msg::new(&MESSAGE_DUMP).unwrap();
    let randtext = commands::RandText::new().unwrap();

    commands::CommandDispatcher::new(vec![
        Box::new(taki),
        Box::new(chain),
        Box::new(poll),
        Box::new(wdyt),
        Box::new(joker),
        Box::new(img2msg),
        Box::new(randtext),
    ])
}

fn bot_help<'a, 'b>(m: &'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a> {
    m.embed(|e| {
        e.color(EMBED_COLOR);
        e.title("Joe's Saloon");
        e.field(
            "таки",
            r#"
`!takistart` — начнем партию
`!takisuspects` — бросим взгляд на плакаты о розыске
`!takistats` — поднимем бокал крепкого виски за самых метких стрелков
                    "#,
            false,
        );
        e.field(
            "мэшап",
            r#"
`!mashup` — узнаем от бармена последние слухи
`!mashupstars` — поприветствуем жителей городка
"#,
            false,
        );
        e.field(
            "политика",
            r#"
`!poll` — устроим честный суд
"#,
            false,
        );
        e.field(
            "поговорим с джо",
            r#"
`что думаешь об итмо и бонче`
`джокер++`
`джокер про итмо`
`джокер про итмо и бонч`
"#,
            false,
        );
        e.field(
            "просто отдохнем",
            r#"
`!random` — попросим бармена налить чего-нибудь покрепче
"#,
            false,
        );
        e.field(
            "займемся делом",
            "_покажи джо фотокарточку, о которой хочешь разузнать побольше_",
            false,
        );
        e
    });
    m
}
