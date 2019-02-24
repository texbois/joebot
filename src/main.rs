use std::collections::HashMap;
use rand::seq::SliceRandom;

mod vk;

fn main() {
    /* https://oauth.vk.com/authorize?client_id=<...>&scope=offline,messages&redirect_uri=https://oauth.vk.com/blank.html&response_type=token */
    let token = std::env::var("API_TOKEN")
        .expect("Provide a valid API token via the API_TOKEN environment variable");
    let chat_id = std::env::var("CHAT_ID")
        .expect("Provide the bot's chatroom id via the CHAT_ID environment variable");

    let vk = vk::Vk::new(token, chat_id);
    
    println!("{:?}", vk.init_long_poll());
}

fn pick_random_target() -> (&'static str, Vec<&'static str>) {
    let mut user_messages: HashMap<&str, Vec<&str>> = HashMap::new();

    user_messages.insert("Somebody", vec!["Once", "Told", "Me", "The", "World", "Is", "Gonna", "Roll", "Me"]);
    user_messages.insert("Joe", vec!["P***", "Get", "R", "Hi", "H", "Oh", "Tex", "Boi"]);
    user_messages.insert("Dave", vec!["But", "Who's", "Buyin", "No", "No No No No"]);

    let users: Vec<&str> = user_messages.keys().map(|&k| k).collect();

    let mut rng = rand::thread_rng();
    let user = users.choose(&mut rng).unwrap();
    let messages: Vec<&str> = user_messages[user].choose_multiple(&mut rng, 5).cloned().collect();

    (user, messages)
}
