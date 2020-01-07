use serde_json::json;

use crate::telegram::{Message, MessageContents, Telegram};

pub fn do_poll<F: FnMut(Message) -> crate::JoeResult<()>>(
    client: &Telegram,
    mut callback: F,
) -> crate::JoeResult<()> {
    let mut update_offset = 0;
    loop {
        let payload = Some(json!({
            "timeout": 25,
            "allowed_updates": ["message"],
            "offset": update_offset + 1
        }));
        let resp: serde_json::Value = client.api_method("getUpdates", payload).send()?.json()?;

        if let Some(last_update_id) = resp["result"]
            .as_array()
            .and_then(|u| u.last())
            .and_then(|u| u["update_id"].as_u64())
        {
            update_offset = last_update_id;
        }

        let messages = resp["result"]
            .as_array()
            .unwrap()
            .into_iter()
            .filter_map(parse_text_message);

        for message in messages {
            callback(message)?;
        }
    }
}

fn parse_text_message(update_obj: &serde_json::Value) -> Option<Message> {
    let message_obj = update_obj.get("message")?;

    let chat_id = message_obj.get("chat")?.get("id")?.as_i64()?;
    let text = message_obj.get("text")?.as_str()?;
    /* If the current message contains a "text" field, it also has { from: { ... } } */

    let from_obj = message_obj.get("from")?;
    let sender = from_obj
        .get("username")
        .and_then(|u| u.as_str())
        .map(|n| n.to_owned())
        .or_else(|| {
            let first_name = from_obj.get("first_name")?.as_str()?;
            let last_name = from_obj.get("last_name")?.as_str()?;
            Some([first_name, " ", last_name].concat())
        })?;

    let bot_command = message_obj
        .get("entities")
        .and_then(|es| es.as_array())
        .and_then(|es| {
            es.iter()
                .find(|e| e["type"] == "bot_command" && e["offset"] == 0)
        })
        .and_then(|e| {
            let cmd_len = e["length"].as_u64().unwrap() as usize;

            match &text[1 /* skip forward slash */..cmd_len]
                .split('@')
                .collect::<Vec<_>>()[..]
            {
                &[cmd] => Some((cmd.to_owned(), None)),
                &[cmd, receiver] => Some((cmd.to_owned(), Some(receiver.to_owned()))),
                _ => None,
            }
        });

    let contents = if let Some((command, receiver)) = bot_command {
        MessageContents::Command { command, receiver }
    } else {
        MessageContents::Text(text.to_owned())
    };

    Some(Message {
        chat_id,
        sender,
        contents,
    })
}
