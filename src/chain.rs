use crate::{telegram, HandlerResult};
use joebot_markov_chain::{ChainGenerate, Datestamp, MarkovChain, TextSource};
use phf::phf_map;
use rand::{rngs::SmallRng, SeedableRng};

static DATE_RANGE_MAP: phf::Map<&'static str, (Datestamp, Datestamp)> = phf_map! {
    "первый курс" => (Datestamp { year: 2017, day: 182 }, Datestamp { year: 2018, day: 182 }),
    "второй курс" => (Datestamp { year: 2018, day: 182 }, Datestamp { year: 2019, day: 182 }),
    "третий курс" => (Datestamp { year: 2019, day: 182 }, Datestamp { year: 2020, day: 183 }),

    "первый сем" => (Datestamp { year: 2017, day: 182 }, Datestamp { year: 2018, day: 28 }),
    "второй сем" => (Datestamp { year: 2018, day: 28 }, Datestamp { year: 2018, day: 182 }),
    "третий сем" => (Datestamp { year: 2018, day: 182 }, Datestamp { year: 2019, day: 28 }),
    "четвертый сем" => (Datestamp { year: 2019, day: 28 }, Datestamp { year: 2019, day: 182 }),
    "пятый сем" => (Datestamp { year: 2019, day: 182 }, Datestamp { year: 2020, day: 28 }),
    "шестой сем" => (Datestamp { year: 2020, day: 28 }, Datestamp { year: 2020, day: 183 }),
};

pub struct Chain<'a> {
    chain: &'a MarkovChain,
    rng: SmallRng,
    last_command: String,
}

impl<'a> Chain<'a> {
    pub fn new(chain: &'a MarkovChain) -> Self {
        Self {
            chain,
            rng: SmallRng::from_entropy(),
            last_command: String::new(),
        }
    }

    pub fn handle_message(&mut self, message: &telegram::Message) -> HandlerResult {
        use crate::telegram::MessageContents::*;

        match &message.contents {
            &Command {
                ref command,
                ref rest,
                ..
            } if command == "mashup" => {
                self.last_command = rest.trim().to_owned();
                HandlerResult::Response(do_mashup(&self.last_command, self.chain, &mut self.rng))
            }
            &Command { ref command, .. } if command == "mashupmore" => {
                HandlerResult::Response(do_mashup(&self.last_command, self.chain, &mut self.rng))
            }
            &Command { ref command, .. } if command == "mashupstars" => {
                HandlerResult::Response(mashup_sources(&self.chain))
            }
            _ => HandlerResult::Unhandled,
        }
    }
}

fn do_mashup(command: &str, chain: &MarkovChain, rng: &mut SmallRng) -> String {
    if command.is_empty() {
        return [
            "\u{2753} Примеры:\n",
            "/mashup joe, ma\n",
            "/mashup joe, овт (первый курс)\n",
            "/mashup joe, ma, осп (пятый сем)",
        ]
        .concat();
    }
    let (names_str, date_range) = if command.chars().last() == Some(')') {
        match command[..command.len() - 1]
            .rsplitn(2, '(')
            .collect::<Vec<_>>()[..]
        {
            [date, names] => match DATE_RANGE_MAP.get(date.trim()) {
                Some(range) => (names, Some(range)),
                _ => {
                    return format!(
                        "\u{274c} {}? Давно это было. Я помню только {}.",
                        date,
                        DATE_RANGE_MAP
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            },
            _ => (command, None),
        }
    } else {
        (command, None)
    };
    match pick_sources(names_str, &chain.sources) {
        Ok(sources) => match date_range {
            Some(range) => chain.generate_in_date_range(rng, sources, *range, 15, 40),
            None => chain.generate(rng, sources, 15, 40),
        }
        .unwrap_or("\u{274c}".into()),
        Err(err) => err,
    }
}

fn mashup_sources(chain: &MarkovChain) -> String {
    format!(
        "* {}\n",
        chain
            .sources
            .iter()
            .map(|s| {
                s.names
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(" / ")
            })
            .collect::<Vec<_>>()
            .join("\n* ")
    )
}

fn pick_sources<'s>(
    names_str: &str,
    sources: &'s Vec<TextSource>,
) -> Result<Vec<&'s TextSource>, String> {
    use alcs::FuzzyStrstr;

    names_str
        .to_lowercase()
        .split(',')
        .map(str::trim)
        .try_fold(Vec::new(), |mut acc, name| {
            let source = sources
                .iter()
                .flat_map(|source| {
                    source.names.iter().map(move |source_name| {
                        source_name
                            .to_lowercase()
                            .fuzzy_find_str(name, 0.5)
                            .map(|(score, _)| (score, source))
                    })
                })
                .flatten()
                .max_by(|(score1, _), (score2, _)| score1.partial_cmp(score2).unwrap());
            source
                .ok_or(format!(
                    "\u{274c} {}? Такого я здесь не встречал, приятель.",
                    name
                ))
                .map(|(_, source)| {
                    acc.push(source);
                    acc
                })
        })
}
