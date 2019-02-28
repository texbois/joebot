mod vk;
mod taki;
mod storage;

include!(concat!(env!("OUT_DIR"), "/messages.rs"));

fn main() {
    /* https://oauth.vk.com/authorize?client_id=<...>&scope=offline,messages&redirect_uri=https://oauth.vk.com/blank.html&response_type=token */
    let token = std::env::var("API_TOKEN")
        .expect("Provide a valid API token via the API_TOKEN environment variable");
    let chat_id: u64 = std::env::var("CHAT_ID").ok().and_then(|id| id.parse().ok())
        .expect("Provide the bot's chatroom id via the CHAT_ID environment variable");

    let redis = storage::Redis::new("redis://127.0.0.1/");

    let vk = vk::Vk::new(token);
    let bot_user = vk.get_bot_user().unwrap();
    let chat_members = vk.get_chat_members(chat_id).unwrap();

    let mut game = taki::Taki::new(chat_id, &bot_user, chat_members, &redis);

    for message in vk.poll_messages().unwrap() {
        if message.sender_id == bot_user.id {
            continue;
        }
        if let Some((dest, reply)) = game.process_with_reply(&message) {
            vk.send_message(dest, reply).unwrap();
        }
    }
}
