use rand::{seq::SliceRandom, Rng};
use std::collections::{HashMap, HashSet};
use vkopt_message_parser::reader::{fold_html, EventResult, MessageEvent};

#[derive(Debug, Clone)]
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
}

impl MessageDump {
    pub fn from_file(input_file: &str, names: &HashSet<&str>) -> Self {
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
                MessageEvent::FullNameExtracted(full_name) if !names.contains(full_name) => {
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

        let word_stem_to_text_idx = build_word_stem_to_text_idx(&texts);

        Self {
            authors,
            texts,
            word_stem_to_text_idx,
        }
    }

    pub fn random_message_with_any_stem<'s, S: AsRef<str>, R: Rng>(
        &'s self,
        stems: &[S],
        rng: &mut R,
    ) -> Option<&'s Message> {
        let text_indexes = stems
            .iter()
            .filter_map(|s| self.word_stem_to_text_idx.get(s.as_ref()))
            .flatten()
            .copied()
            .collect::<Vec<u32>>();
        text_indexes
            .choose(rng)
            .map(|&idx| &self.texts[idx as usize])
    }

    pub fn random_message_with_all_stems<'s, S: AsRef<str>, R: Rng>(
        &'s self,
        stems: &[S],
        rng: &mut R,
    ) -> Option<&'s Message> {
        let stem_indexes = stems
            .iter()
            .filter_map(|s| {
                self.word_stem_to_text_idx
                    .get(s.as_ref())
                    .map(|idxs| idxs.iter().copied().collect::<HashSet<u32>>())
            })
            .collect::<Vec<HashSet<u32>>>();

        let mut indexes_with_all_stems = Vec::new();
        for idx in &stem_indexes[0] {
            if stem_indexes.iter().all(|s| s.contains(&idx)) {
                indexes_with_all_stems.push(*idx);
            }
        }

        indexes_with_all_stems
            .choose(rng)
            .map(|&idx| &self.texts[idx as usize])
    }
}

fn build_word_stem_to_text_idx(texts: &Vec<Message>) -> HashMap<String, Vec<u32>> {
    let mut map: HashMap<String, Vec<u32>> = HashMap::new();

    let en_stemmer = rust_stemmers::Stemmer::create(rust_stemmers::Algorithm::English);
    let ru_stemmer = rust_stemmers::Stemmer::create(rust_stemmers::Algorithm::Russian);

    for (idx, msg) in texts.iter().enumerate() {
        if msg.text.chars().count() >= 2000 {
            /* Exceeds the limit set by Discord */
            continue;
        }
        for word in msg.text.split_ascii_whitespace() {
            let stemmer = if word.is_ascii() {
                &en_stemmer
            } else {
                &ru_stemmer
            };
            let stem: String = stemmer.stem(word).into();

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
