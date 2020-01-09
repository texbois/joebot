use crate::{telegram, HandlerResult};
use joebot_markov_chain::{ChainGenerate, Datestamp, MarkovChain};
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

pub struct Chain {
    rng: SmallRng,
}

impl Chain {
    pub fn new() -> Self {
        Self {
            rng: SmallRng::from_entropy(),
        }
    }

    pub fn handle_command(
        &mut self,
        message: &telegram::Message,
        chain: &MarkovChain,
    ) -> HandlerResult {
        use crate::telegram::MessageContents::*;

        match &message.contents {
            &Command {
                ref command,
                ref rest,
                ..
            } if command == "mashup" => {
                HandlerResult::Response(do_mashup(rest.trim(), chain, &mut self.rng))
            }
            _ => HandlerResult::Unhandled,
        }
    }
}

fn do_mashup(command: &str, chain: &MarkovChain, rng: &mut SmallRng) -> String {
    if command.is_empty() {
        return [
            "Примеры:\n",
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
            [names, date] => match DATE_RANGE_MAP.get(date.trim()) {
                Some(range) => (names, Some(range)),
                _ => {
                    return format!(
                        "{}? Давно это было. Я помню только {}.",
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
    let names = names_str.split(',').map(|n| n.trim()).collect::<Vec<_>>();
    match date_range {
        Some(range) => chain.generate_in_date_range(rng, &names, *range, 20),
        None => chain.generate(rng, &names, 20),
    }
    .unwrap_or("-".into())
}
