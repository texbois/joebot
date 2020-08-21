use crate::{utils::split_command_rest, JoeResult};
use joebot_markov_chain::{ChainGenerate, Datestamp, MarkovChain, Selector, SelectorError};
use phf::phf_map;
use rand::{rngs::SmallRng, SeedableRng};
use serenity::{builder::CreateMessage, model::prelude::*, prelude::*};

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
    chain: MarkovChain,
    rng: SmallRng,
}

impl Chain {
    pub fn new(chain: MarkovChain) -> Self {
        Self {
            chain,
            rng: SmallRng::from_entropy(),
        }
    }
}

fn chain_help<'a, 'b>(m: &'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a> {
    m.embed(|e| {
        e.color(crate::EMBED_COLOR);
        e.title("мэшап");
        e.description(
            r#"
Выбери источники, от которых хочешь услышать сплетни.
Список всех источников — `!mashupstars`

Текст от одного из источников a, b, c:
`!mashup a | b | c`

Текст от всех источников a, b, c одновременно:
`!mashup a & b & c`

Сложные селекторы удовольствия:
`!mashup (a | b) & (c | d)`

Ограничение источника по времени:
`!mashup a | b [шестой сем]`
`!mashup a | b [третий курс]`"#,
        );
        e
    });
    m
}

fn chain_invalid_date_range<'a, 'b>(
    d: &'a str,
    m: &'b mut CreateMessage<'a>,
) -> &'b mut CreateMessage<'a> {
    m.embed(|e| {
        e.color(crate::EMBED_COLOR);
        e.title(format!("{}? Давно это было.", d));
        e.description(format!(
            "Спроси меня про {}",
            DATE_RANGE_MAP
                .keys()
                .copied()
                .collect::<Vec<_>>()
                .join(", ")
        ));
        e
    });
    m
}

fn chain_selector_error<'a, 'b>(
    e: SelectorError,
    m: &'b mut CreateMessage<'a>,
) -> &'b mut CreateMessage<'a> {
    let msg = match e {
        SelectorError::EmptyQuery => return chain_help(m),
        SelectorError::ParserExpectedTerm { location } => format!(
            "Мой железный бык нашептал мне, что он ожидал увидеть имя вот здесь: {}",
            location
        ),
        SelectorError::ParserUnbalancedParentheses { location } => format!(
            "Мой железный бык нашептал мне, что у тебя не закрыты скобки: {}",
            location
        ),
        SelectorError::UnknownTerm { term } => format!(
            "Мой железный бык нашептал мне, что про \"{}\" в этих краях не слыхали.",
            term
        ),
    };
    m.embed(|e| {
        e.color(crate::EMBED_COLOR);
        e.title("Неправильный запрос, приятель.");
        e.description(msg);
        e
    });
    m
}

fn chain_sources<'a, 'b>(
    c: &MarkovChain,
    m: &'b mut CreateMessage<'a>,
) -> &'b mut CreateMessage<'a> {
    m.embed(|e| {
        e.color(crate::EMBED_COLOR);
        e.title("мэшапстарс");
        e.description(format!(
            "* {}\n",
            c.sources
                .iter()
                .map(|s| s.name_re.as_str())
                .collect::<Vec<_>>()
                .join("\n* ")
        ));
        e
    });
    m
}

impl super::Command for Chain {
    fn handle_message(&mut self, ctx: &Context, msg: &Message) -> JoeResult<bool> {
        let (command, rest) = split_command_rest(msg);
        match command {
            "!mashup" => {
                let mashup_cmd = rest.trim();
                if mashup_cmd.is_empty() {
                    msg.channel_id.send_message(&ctx.http, chain_help)?;
                    return Ok(true);
                }
                let (names_str, date_range) = if rest.ends_with(']') {
                    match rest[..rest.len() - 1].rsplitn(2, '[').collect::<Vec<_>>()[..] {
                        [date, names] => match DATE_RANGE_MAP.get(date.trim()) {
                            Some(range) => (names, Some(range.clone())),
                            _ => {
                                msg.channel_id.send_message(&ctx.http, |m| {
                                    chain_invalid_date_range(date, m)
                                })?;
                                return Ok(true);
                            }
                        },
                        _ => (rest, None),
                    }
                } else {
                    (rest, None)
                };
                match Selector::new(&self.chain, names_str, date_range) {
                    Ok(selector) => {
                        let text = do_mashup(&self.chain, &mut self.rng, &selector);
                        msg.channel_id.say(&ctx.http, text)?;
                    }
                    Err(e) => {
                        msg.channel_id
                            .send_message(&ctx.http, |m| chain_selector_error(e, m))?;
                    }
                };
                Ok(true)
            }
            "!mashupmore" => {
                msg.channel_id.say(&ctx.http, "Unsupported")?;
                Ok(true)
            }
            "!mashupstars" => {
                msg.channel_id
                    .send_message(&ctx.http, |m| chain_sources(&self.chain, m))?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn handle_reaction(&mut self, _ctx: &Context, rct: &Reaction) -> JoeResult<bool> {
        match &rct.emoji {
            ReactionType::Unicode(e) if e == "🔁" => {
                println!("Received repeat reaction {:?}", rct);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

fn do_mashup(chain: &MarkovChain, rng: &mut SmallRng, selector: &Selector) -> String {
    chain
        .generate(selector, rng, 15, 40)
        .unwrap_or_else(|| String::from(r"¯\_(ツ)_/¯"))
}
