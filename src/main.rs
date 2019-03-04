#![feature(slice_patterns)]

mod taki;
mod telegram;
mod storage;

include!(concat!(env!("OUT_DIR"), "/messages.rs"));

fn main() {
    let bot_token = std::env::var("BOT_TOKEN")
        .expect("Provide a valid bot token via the BOT_TOKEN environment variable");
    let bot_chat_id: i64 = std::env::var("CHAT_ID").ok().and_then(|id| id.parse().ok())
        .expect("Provide the bot's chatroom id via the CHAT_ID environment variable");

    let redis = storage::Redis::new("redis://127.0.0.1/");

    let telegram = telegram::Telegram::new(&bot_token);
    let bot_name = telegram.get_bot_username().unwrap();

    println!("@{} is ready. Polling for incoming messages from chat #{}", bot_name, bot_chat_id);

    let mut game = taki::Taki::new(bot_chat_id, &redis);

    for message in telegram.poll_messages() {
        if message.chat_id != bot_chat_id {
            continue;
        }
        if let telegram::MessageContents::Command { receiver: Some(ref receiver_name), .. } = message.contents {
            if receiver_name != &bot_name {
                continue;
            }
        }

        if let Some(reply) = game.process_with_reply(&message) {
            telegram.send_message(bot_chat_id, &reply).unwrap();
        }
    }
}
