use vkopt_message_parser::reader::{fold_html, EventResult, MessageEvent};

#[derive(Debug)]
pub struct Author {
    pub short_name: String,
    pub full_name: String,
}

#[derive(Debug)]
pub struct Message {
    pub text: String,
    pub author_idx: usize,
}

#[derive(Debug)]
pub struct MessageDump {
    pub authors: Vec<Author>,
    pub texts: Vec<Message>,
}

impl MessageDump {
    pub fn from_file<S: AsRef<str>>(input_file: &str, ignore_names: &[S]) -> Self {
        let mut authors: Vec<Author> = Vec::new();
        let texts = fold_html(
            input_file,
            Vec::new(),
            |mut msgs: Vec<Message>, event| match event {
                MessageEvent::Start(_) => match msgs.last_mut() {
                    Some(msg) if msg.text.is_empty() => {
                        msg.author_idx = 0;
                        EventResult::Consumed(msgs)
                    }
                    _ => {
                        msgs.push(Message {
                            text: String::new(),
                            author_idx: 0,
                        });
                        EventResult::Consumed(msgs)
                    }
                },
                MessageEvent::FullNameExtracted(full_name)
                    if ignore_names.iter().any(|n| n.as_ref() == full_name) =>
                {
                    EventResult::SkipMessage(msgs)
                }
                MessageEvent::FullNameExtracted(full_name) => {
                    msgs.last_mut().unwrap().author_idx = authors
                        .iter()
                        .enumerate()
                        .find(|(_, a)| a.full_name == full_name)
                        .map(|(i, _)| i)
                        .unwrap_or_else(|| {
                            authors.push(Author {
                                full_name: full_name.to_owned(),
                                short_name: String::new(),
                            });
                            authors.len() - 1
                        });
                    EventResult::Consumed(msgs)
                }
                MessageEvent::ShortNameExtracted(short_name) => {
                    let author_idx = msgs.last_mut().unwrap().author_idx;
                    if authors[author_idx].short_name.is_empty() {
                        authors[author_idx].short_name.push_str(short_name);
                    }
                    EventResult::Consumed(msgs)
                }
                MessageEvent::BodyPartExtracted(body) => {
                    msgs.last_mut().unwrap().text.push_str(body);
                    EventResult::Consumed(msgs)
                }
                _ => EventResult::Consumed(msgs),
            },
        )
        .unwrap();
        Self { authors, texts }
    }
}
