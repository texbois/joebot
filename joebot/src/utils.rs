use serenity::model::prelude::*;

pub fn split_command_rest(msg: &Message) -> (&str, &str) {
    match msg.content.splitn(2, ' ').collect::<Vec<&str>>()[..] {
        [command, rest] => (command, rest.trim()),
        _ => (&msg.content, ""),
    }
}
