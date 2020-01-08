use std::error::Error;

pub type JoeResult<T> = Result<T, Box<dyn Error>>;

mod messages;
mod storage;
mod taki;
mod telegram;

fn main() {
    let bot_token = std::env::var("BOT_TOKEN")
        .expect("Provide a valid bot token via the BOT_TOKEN environment variable");
    let bot_chat_id: i64 = std::env::var("CHAT_ID")
        .ok()
        .and_then(|id| id.parse().ok())
        .expect("Provide the bot's chatroom id via the CHAT_ID environment variable");

    match run(bot_token, bot_chat_id) {
        Ok(_) => println!("Good night, sweet prince."),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn run(bot_token: String, bot_chat_id: i64) -> JoeResult<()> {
    let messages = messages::MessageDump::from_file("messages.html");
    let mut redis =
        storage::Redis::new("redis://127.0.0.1/").map_err(|e| format!("redis: {}", e))?;

    let telegram = telegram::Telegram::new(&bot_token);
    let bot_name = telegram.get_bot_username()?;

    println!(
        "@{} is ready. Polling for incoming messages from chat #{}",
        bot_name, bot_chat_id
    );

    let mut game = taki::Taki::new(&messages, bot_chat_id, &mut redis);

    telegram.poll_messages(|message| {
        if message.chat_id != bot_chat_id {
            return Ok(());
        }
        if let telegram::MessageContents::Command {
            receiver: Some(ref receiver_name),
            ..
        } = message.contents
        {
            if receiver_name != &bot_name {
                return Ok(());
            }
        }

        if let Some(reply) = game.process_with_reply(&message) {
            telegram.send_message(bot_chat_id, &reply)?;
        }
        Ok(())
    })
}
