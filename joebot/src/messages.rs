use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use vkopt_message_parser::reader::{fold_html, EventResult, MessageEvent};

const DISCORD_TEXT_LIMIT: usize = 2000;

#[derive(Debug, Clone, PartialEq)]
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
    word_stem_to_text_idx: HashMap<String, Vec<u32>>,
    stemmer: Stemmer,
}

impl MessageDump {
    pub fn from_file(input_file: &str, names: &HashSet<&str>) -> Self {
        let mut authors: Vec<Author> = Vec::new();
        let mut last_full_name: String = String::new();

        let texts = fold_html(
            input_file,
            Vec::new(),
            |mut msgs: Vec<Message>, event| match event {
                MessageEvent::Start(_) => match msgs.last_mut() {
                    Some(msg)
                        if msg.text.trim().is_empty()
                            || msg.text.chars().count() >= DISCORD_TEXT_LIMIT =>
                    {
                        msg.text.clear();
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
                MessageEvent::FullNameExtracted(full_name) => {
                    last_full_name.clear();
                    last_full_name.push_str(full_name);
                    EventResult::Consumed(msgs)
                }
                MessageEvent::ShortNameExtracted(short_name) if !names.contains(short_name) => {
                    EventResult::SkipMessage(msgs)
                }
                MessageEvent::ShortNameExtracted(short_name) => {
                    msgs.last_mut().unwrap().author_idx = authors
                        .iter()
                        .position(|a| a.short_name == short_name)
                        .unwrap_or_else(|| {
                            authors.push(Author {
                                full_name: last_full_name.to_owned(),
                                short_name: short_name.to_owned(),
                            });
                            authors.len() - 1
                        });
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

        let stemmer = Stemmer::new();
        let word_stem_to_text_idx = build_word_stem_to_text_idx(&texts, &stemmer);

        Self {
            authors,
            texts,
            word_stem_to_text_idx,
            stemmer,
        }
    }

    pub fn containing_any_words<'p, P: Prompt>(&self, prompt: &'p P) -> Vec<&Message> {
        prompt
            .stems(&self.stemmer)
            .into_iter()
            .filter_map(move |s| self.word_stem_to_text_idx.get(s.as_ref()))
            .flatten()
            .map(|idx| &self.texts[*idx as usize])
            .collect()
    }

    pub fn containing_all_words<'p, P: Prompt>(&'p self, prompt: &'p P) -> Vec<&Message> {
        let stem_indexes = prompt
            .stems(&self.stemmer)
            .iter()
            .filter_map(|s| {
                self.word_stem_to_text_idx
                    .get(s.as_ref())
                    .map(|idxs| idxs.iter().copied().collect::<HashSet<u32>>())
            })
            .collect::<Vec<HashSet<u32>>>();

        if stem_indexes.is_empty() {
            return vec![];
        }

        let mut messages_with_all_stems = Vec::new();
        for idx in &stem_indexes[0] {
            if stem_indexes.iter().all(|s| s.contains(&idx)) {
                messages_with_all_stems.push(&self.texts[*idx as usize]);
            }
        }
        messages_with_all_stems
    }
}

fn build_word_stem_to_text_idx(texts: &[Message], stemmer: &Stemmer) -> HashMap<String, Vec<u32>> {
    let mut map: HashMap<String, Vec<u32>> = HashMap::new();

    for (idx, msg) in texts.iter().enumerate() {
        for stem in split_text_into_stems(&msg.text, stemmer) {
            match map.get_mut(&stem) {
                Some(indexes) => {
                    indexes.push(idx as u32);
                }
                None => {
                    map.insert(stem, vec![idx as u32]);
                }
            }
        }
    }

    map
}

pub struct Stemmer {
    ru_stemmer: rust_stemmers::Stemmer,
    en_stemmer: rust_stemmers::Stemmer,
}

impl std::fmt::Debug for Stemmer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Stemmer")
    }
}

impl Stemmer {
    fn new() -> Self {
        let ru_stemmer = rust_stemmers::Stemmer::create(rust_stemmers::Algorithm::Russian);
        let en_stemmer = rust_stemmers::Stemmer::create(rust_stemmers::Algorithm::English);
        Self {
            ru_stemmer,
            en_stemmer,
        }
    }

    pub fn stem<'a>(&self, input: &'a str) -> Cow<'a, str> {
        if input.is_ascii() {
            self.en_stemmer.stem(&input)
        } else {
            self.ru_stemmer.stem(&input)
        }
    }
}

pub trait Prompt {
    fn stems<'a>(&'a self, stemmer: &Stemmer) -> Vec<Cow<'a, str>>;
}

impl Prompt for &str {
    fn stems(&self, stemmer: &Stemmer) -> Vec<Cow<str>> {
        split_text_into_stems(self, stemmer)
            .map(move |w| Cow::Owned(w))
            .collect::<Vec<_>>()
    }
}

impl Prompt for &[&str] {
    fn stems(&self, _: &Stemmer) -> Vec<Cow<str>> {
        self.iter().map(|s| Cow::Borrowed(*s)).collect()
    }
}

fn split_text_into_stems<'t>(
    text: &'t str,
    stemmer: &'t Stemmer,
) -> impl Iterator<Item = String> + 't {
    text.split(&[' ', '\n', '.', '…', ',', '!', '?', '(', ')', '[', ']', '/', '|', '@', '"', ':', '-', '+'][..])
        .filter_map(move |w| {
            let word = w.trim();
            if word.is_empty() {
                None
            } else {
                Some(stemmer.stem(&w.to_lowercase()).into_owned())
            }
        })
}
