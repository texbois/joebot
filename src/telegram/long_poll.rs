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
        if let Some(update_id) = process_poll_response(resp, &mut callback)? {
            update_offset = update_id;
        }
    }
}

fn process_poll_response<F: FnMut(Message) -> crate::JoeResult<()>>(
    mut resp: serde_json::Value,
    callback: &mut F,
) -> crate::JoeResult<Option<u64>> {
    let updates = match resp["result"].take() {
        serde_json::Value::Array(entries) => entries,
        _ => return Err(format!("Invalid response: {}", resp).into()),
    };

    let update_id = if let Some(last_update) = updates.last() {
        last_update["update_id"]
            .as_u64()
            .ok_or(format!("Missing update_id: {}", resp))?
    } else {
        return Ok(None);
    };

    for message in updates.into_iter().filter_map(parse_text_message) {
        callback(message)?;
    }

    Ok(Some(update_id))
}

fn parse_text_message(mut update_obj: serde_json::Value) -> Option<Message> {
    let mut message_obj = match update_obj["message"].take() {
        serde_json::Value::Object(obj) => obj,
        _ => return None,
    };

    let chat_id = message_obj["chat"]["id"].as_i64()?;

    let text = match message_obj.get_mut("text")?.take() {
        serde_json::Value::String(text) => text,
        _ => return None,
    };

    let sender = match message_obj["from"]["username"].take() {
        serde_json::Value::String(username) => username,
        _ => [
            message_obj["from"]["first_name"].as_str()?,
            " ",
            message_obj["from"]["last_name"].as_str()?,
        ]
        .concat(),
    };

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
                &[cmd] => Some((cmd.to_owned(), None, text[cmd_len..].to_owned())),
                &[cmd, receiver] => Some((
                    cmd.to_owned(),
                    Some(receiver.to_owned()),
                    text[cmd_len..].to_owned(),
                )),
                _ => None,
            }
        });

    let contents = if let Some((command, receiver, rest)) = bot_command {
        MessageContents::Command {
            command,
            receiver,
            rest,
        }
    } else {
        MessageContents::Text(text)
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
    fn test_poll_empty() {
        let resp = json!({"ok": true, "result": []});
        let mut messages: Vec<Message> = Vec::new();
        let update_id = process_poll_response(resp, &mut |msg| Ok(messages.push(msg))).unwrap();
        assert!(update_id.is_none());
        assert!(messages.is_empty());
    }

    #[test]
    fn test_poll_message() {
        let resp = json!({"ok": true, "result": [
            {
                "message": {
                    "chat": {
                        "first_name": "Jill",
                        "id": 100,
                        "type": "private",
                        "username": "Shadowmaster69"
                    },
                    "date": 3249849600i64,
                    "from": {
                        "first_name": "Jill",
                        "id": 100,
                        "is_bot": false,
                        "language_code": "en",
                        "username": "Shadowmaster69"
                    },
                    "message_id": 1000,
                    "text": "ice cream"
                },
                "update_id": 10000
            }
        ]});
        let mut messages: Vec<Message> = Vec::new();
        let update_id = process_poll_response(resp, &mut |msg| Ok(messages.push(msg))).unwrap();
        assert_eq!(update_id, Some(10000));
        assert_eq!(
            messages,
            vec![Message {
                chat_id: 100,
                sender: "Shadowmaster69".into(),
                contents: MessageContents::Text("ice cream".into())
            }]
        );
    }

    #[test]
    fn test_poll_bot_command() {
        let resp = json!({"ok": true, "result": [
            {
                "message": {
                    "chat": {
                        "first_name": "Dana",
                        "last_name": "Zane",
                        "id": 200,
                        "type": "private",
                    },
                    "date": 3249849800i64,
                    "entities": [
                        {"length": 15, "offset": 0, "type": "bot_command"}
                    ],
                    "from":{
                        "first_name": "Dana",
                        "last_name": "Zane",
                        "id": 200,
                        "is_bot": false,
                        "language_code": "en",
                    },
                    "message_id": 3000,
                    "text":"/start@corgibot"
                },
                "update_id": 10000
            },
            {
                "message": {
                    "chat": {
                        "first_name": "Dana",
                        "last_name": "Zane",
                        "id": 200,
                        "type": "private",
                    },
                    "date": 3249860000i64,
                    "entities":[
                        {"length": 6, "offset": 0, "type" :"bot_command"}
                    ],
                    "from": {
                        "first_name": "Dana",
                        "last_name": "Zane",
                        "id": 200,
                        "is_bot": false,
                        "language_code": "en",
                    },
                    "message_id": 3001,
                    "text": "/start microwave"
                },
                "update_id": 10001
            }
        ]});
        let mut messages: Vec<Message> = Vec::new();
        let update_id = process_poll_response(resp, &mut |msg| Ok(messages.push(msg))).unwrap();
        assert_eq!(update_id, Some(10001));
        assert_eq!(
            messages,
            vec![
                Message {
                    chat_id: 200,
                    sender: "Dana Zane".into(),
                    contents: MessageContents::Command {
                        command: "start".into(),
                        receiver: Some("corgibot".into()),
                        rest: String::new()
                    }
                },
                Message {
                    chat_id: 200,
                    sender: "Dana Zane".into(),
                    contents: MessageContents::Command {
                        command: "start".into(),
                        receiver: None,
                        rest: " microwave".into()
                    }
                }
            ]
        );
    }

    #[test]
    fn test_message_without_text() {
        let resp = json!({"ok": true, "result": [
            {
                "message": {
                    "chat": {
                        "first_name": "Jill",
                        "id": 100,
                        "type": "private",
                        "username": "Shadowmaster69"
                    },
                    "date": 3249849600i64,
                    "from": {
                        "first_name": "Jill",
                        "id": 100,
                        "is_bot": false,
                        "language_code": "en",
                        "username": "Shadowmaster69"
                    },
                    "message_id": 1000,
                    "photo": [
                        {"file_id":"h","file_size": 30933, "file_unique_id": "hh", "height": 320, "width": 276}
                    ]
                },
                "update_id": 10000
            }
        ]});
        let mut messages: Vec<Message> = Vec::new();
        let update_id = process_poll_response(resp, &mut |msg| Ok(messages.push(msg))).unwrap();
        assert!(messages.is_empty());
        assert_eq!(update_id, Some(10000));
    }
}
