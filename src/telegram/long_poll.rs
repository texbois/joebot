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
        update_offset = process_poll_response(resp, &mut callback)?;
    }
}

fn process_poll_response<F: FnMut(Message) -> crate::JoeResult<()>>(
    mut resp: serde_json::Value,
    callback: &mut F,
) -> crate::JoeResult<u64> {
    let last_update_id = resp["result"]
        .as_array()
        .and_then(|u| u.last())
        .and_then(|u| u["update_id"].as_u64())
        .ok_or(format!(
            "Long poll response does not contain \"update_id\": {}",
            resp
        ))?;

    let messages = match resp["result"].take() {
        serde_json::Value::Array(msgs) => msgs.into_iter().filter_map(parse_text_message),
        res => {
            return Err(format!(
                "Invalid long poll [\"result\"]: {}\nFull response: {}",
                res, resp
            )
            .into())
        }
    };

    for message in messages {
        callback(message)?;
    }

    Ok(last_update_id)
}

fn parse_text_message(mut update_obj: serde_json::Value) -> Option<Message> {
    let mut message_obj = update_obj.get_mut("message")?.take();

    let chat_id = message_obj.get("chat")?.get("id")?.as_i64()?;
    let text = match message_obj.get_mut("text").map(serde_json::Value::take) {
        Some(serde_json::Value::String(text)) => Some(text),
        _ => None,
    }?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poll_message() {
        let resp = json!({
            "ok": true,
            "result": [
                {
                    "message": {
                        "chat": {
                            "first_name": "Jill",
                            "id": 100,
                            "type": "private",
                            "username": "changelivesjill"
                        },
                        "date": 3249849600i64,
                        "from": {
                            "first_name": "Jill",
                            "id": 100,
                            "is_bot": false,
                            "language_code": "en",
                            "username": "changelivesjill"
                        },
                        "message_id": 1000,
                        "text": "hey"
                    },
                    "update_id": 10000
                }
            ]
        });
        let mut messages: Vec<Message> = Vec::new();
        let update_id = process_poll_response(resp, &mut |msg| Ok(messages.push(msg))).unwrap();
        assert_eq!(update_id, 10000);
        assert_eq!(
            messages,
            vec![Message {
                chat_id: 100,
                sender: "changelivesjill".into(),
                contents: MessageContents::Text("hey".into())
            }]
        );
    }
}
