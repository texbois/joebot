use std::collections::HashMap;
use rand::seq::SliceRandom;

fn main() {
    let (user, messages) = pick_random_target();
    println!("{} -> {:?}", user, messages);
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
