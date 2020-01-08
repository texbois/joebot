use std::error::Error;

pub type JoeResult<T> = Result<T, Box<dyn Error>>;

mod messages;
mod storage;
mod taki;
mod telegram;

struct JoeConfig {
    bot_token: String,
    bot_chat_id: i64,
    messages: messages::MessageDump,
}

fn main() {
    let bot_token = std::env::var("BOT_TOKEN")
        .expect("Provide a valid bot token via the BOT_TOKEN environment variable");
    let bot_chat_id: i64 = std::env::var("CHAT_ID")
        .ok()
        .and_then(|id| id.parse().ok())
        .expect("Provide the bot's chatroom id via the CHAT_ID environment variable");
    let taki_ignore_names: Vec<String> = std::env::var("TAKI_IGNORE_NAMES")
        .map(|v| v.split(",").map(|n| n.to_owned()).collect())
        .unwrap_or(Vec::new());

    let messages = messages::MessageDump::from_file("messages.html", &taki_ignore_names);
    let message_authors = messages
        .authors
        .iter()
        .map(|a| a.full_name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "Extracted {} messages from the following authors: {}",
        messages.texts.len(),
        message_authors
    );

    let config = JoeConfig {
        bot_token,
        bot_chat_id,
        messages,
    };

    match run(&config) {
        Ok(_) => println!("Good night, sweet prince."),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn run(config: &JoeConfig) -> JoeResult<()> {
    let mut redis =
        storage::Redis::new("redis://127.0.0.1/").map_err(|e| format!("redis: {}", e))?;

    let telegram = telegram::Telegram::new(&config.bot_token);
    let bot_name = telegram.get_bot_username()?;

    println!(
        "@{} is ready. Polling for incoming messages from chat #{}",
        bot_name, config.bot_chat_id
    );

    let mut game = taki::Taki::new(&config.messages, config.bot_chat_id, &mut redis);

    telegram.poll_messages(|message| match message {
        telegram::Message { chat_id, .. } if chat_id != config.bot_chat_id => Ok(()),
        telegram::Message {
            contents:
                telegram::MessageContents::Command {
                    receiver: Some(ref receiver_name),
                    ..
                },
            ..
        } if receiver_name != &bot_name => Ok(()),
        msg => {
            if let Some(reply) = game.process_with_reply(&msg) {
                telegram.send_message(bot_chat_id, &reply)?;
            }
            Ok(())
        }
    })
}
