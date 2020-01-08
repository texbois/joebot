use crate::{telegram, HandlerResult};
use joebot_markov_chain::chain::MarkovChain;

pub fn handle_command(message: &telegram::Message, chain: &MarkovChain) -> HandlerResult {
    use crate::telegram::MessageContents::*;

    match &message.contents {
        &Command {
            ref command,
            ref rest,
            ..
        } if command == "mashup" => HandlerResult::Response(do_mashup(rest, chain)),
        _ => HandlerResult::Unhandled,
    }
}

fn do_mashup(command: &str, chain: &MarkovChain) -> String {
    String::from("???")
}
