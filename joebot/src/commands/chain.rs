use crate::{utils::split_command_rest, JoeResult};
use joebot_markov_chain::{ChainGenerate, Datestamp, MarkovChain, TextSource};
use phf::phf_map;
use rand::{rngs::SmallRng, SeedableRng};
use serenity::{model::prelude::*, prelude::*};

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

pub struct Chain {
    chain: joebot_markov_chain::MarkovChain,
    rng: SmallRng,
    last_command: String,
}

impl Chain {
    pub fn new(chain: MarkovChain) -> Self {
        Self {
            chain,
            rng: SmallRng::from_entropy(),
            last_command: String::new(),
        }
    }
}

impl super::Command for Chain {
    fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
        let (command, rest) = split_command_rest(msg);
        let resp = match command {
            "!mashup" => {
                self.last_command = rest.trim().to_owned();
                Some(do_mashup(&self.last_command, &self.chain, &mut self.rng))
            }
            "!mashupmore" => Some(do_mashup(&self.last_command, &self.chain, &mut self.rng)),
            "!mashupstars" => Some(mashup_sources(&self.chain, rest)),
            _ => None,
        };
        if let Some(r) = resp {
            msg.channel_id.say(&ctx.http, r)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

fn do_mashup(command: &str, chain: &MarkovChain, rng: &mut SmallRng) -> String {
    if command.is_empty() {
        return [
            "\u{2753} Примеры:\n",
            "!mashup joe, ma\n",
            "!mashup joe, етестер (пятый сем)\n",
            "!mashup joe, ma, овт (первый курс)",
        ]
        .concat();
    }
    let (names_str, date_range) = if command.ends_with(')') {
        match command[..command.len() - 1]
            .rsplitn(2, '(')
            .collect::<Vec<_>>()[..]
        {
            [date, names] => match DATE_RANGE_MAP.get(date.trim()) {
                Some(range) => (names, Some(range.clone())),
                _ => {
                    return format!(
                        "\u{274c} {}? Давно это было. Я помню только {}.",
                        date,
                        DATE_RANGE_MAP
                            .keys()
                            .copied()
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
    match joebot_markov_chain::Selector::new(&chain.sources, names_str, date_range) {
        Ok(selector) =>
            chain.generate(rng, &chain.sources, &selector, 15, 40).unwrap_or_else(|| String::from("\u{274c}")),
        Err(joebot_markov_chain::SelectorError::EmptyQuery) =>
            "Пустой запрос, приятель.".into(),
        Err(joebot_markov_chain::SelectorError::ParserExpectedTerm { location }) =>
            format!("Неправильный запрос, приятель.\nМой железный бык нашептал мне, что он ожидал увидеть имя вот здесь: {}", location),
        Err(joebot_markov_chain::SelectorError::ParserUnbalancedParentheses { location }) =>
            format!("Неправильный запрос, приятель.\nМой железный бык нашептал мне, что у тебя незакрыты скобки: {}", location),
        Err(joebot_markov_chain::SelectorError::UnknownTerm { term }) =>
            format!("\u{274c} {}? Такого я здесь не встречал, приятель.", term)
    }
}

fn mashup_sources(chain: &MarkovChain, filter: &str) -> String {
    let sources = if !filter.is_empty() {
        match pick_sources(filter, &chain.sources) {
            Ok(sources) => sources,
            Err(e) => return e,
        }
    } else {
        chain.sources.iter().collect::<Vec<_>>()
    };
    format!(
        "* {}\n",
        sources
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
    sources: &'s [TextSource],
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
                        let source_name_lower = source_name.to_lowercase();
                        if name == source_name_lower {
                            Some((1.0, source))
                        } else {
                            source_name_lower
                                .fuzzy_find_pos(name, 0.5)
                                .map(|(score, _, _)| (score, source))
                        }
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
