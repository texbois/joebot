use bincode;
use joebot_markov_chain::{ChainAppend, Datestamp, MarkovChain};
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;

fn main() {
    if !std::path::Path::new("chain.bin").exists() {
        let sources: Vec<ChainSource> =
            serde_json::from_str(&std::fs::read_to_string("chain_sources.json").unwrap()).unwrap();
        println!(
            "Joebot build: chain.bin does not exist, will be created from {:?}",
            sources
        );
        build_chain_bin(sources);
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ChainSource {
    MessageDump {
        path: String,
        short_name_regexes: HashMap<String, String>,
    },
    Text {
        path: String,
        name_regex: String,
        year: i16,
        day: u16,
    },
}

fn build_chain_bin(sources: Vec<ChainSource>) {
    let mut chain = MarkovChain::new();
    for src in sources.into_iter() {
        match src {
            ChainSource::MessageDump {
                path,
                short_name_regexes,
            } => {
                let re_map = short_name_regexes
                    .into_iter()
                    .map(|(n, re)| (n, Regex::new(&re).unwrap()))
                    .collect::<HashMap<_, _>>();
                chain.append_message_dump(&path, &re_map);
            }
            ChainSource::Text {
                path,
                name_regex,
                year,
                day,
            } => chain.append_text(
                &path,
                Regex::new(&name_regex).unwrap(),
                Datestamp { year, day },
            ),
        }
    }
    println!(
        "{:?}",
        chain
            .sources
            .iter()
            .map(|s| s.name_re.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "{} chain entries, {} bytes occupied by entries",
        chain.sources.iter().map(|s| s.entries.len()).sum::<usize>(),
        chain
            .sources
            .iter()
            .map(|s| s.entries.capacity())
            .sum::<usize>()
            * std::mem::size_of::<joebot_markov_chain::ChainEntry>()
    );
    bincode::serialize_into(&File::create("chain.bin").unwrap(), &chain).unwrap();
}
