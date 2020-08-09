use bincode;
use joebot_markov_chain::{ChainAppend, Datestamp, MarkovChain};
use serde::Deserialize;
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
    },
    Text {
        path: String,
        names: Vec<String>,
        year: i16,
        day: u16,
    },
}

fn build_chain_bin(sources: Vec<ChainSource>) {
    let mut chain = MarkovChain::new();
    for src in sources.into_iter() {
        match src {
            ChainSource::MessageDump { path } => chain.append_message_dump(&path),
            ChainSource::Text {
                path,
                names,
                year,
                day,
            } => chain.append_text(&path, names, Datestamp { year, day }),
        }
    }
    println!(
        "{:?}",
        chain
            .sources
            .iter()
            .filter_map(|s| s.names.iter().nth(0).map(|s| s.as_str()))
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
