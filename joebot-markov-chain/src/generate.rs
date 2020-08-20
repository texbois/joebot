use crate::{ChainEntry, MarkovChain, Selector};
use indexmap::IndexSet;
use rand::Rng;
use std::collections::HashSet;

const MAX_TRIES: usize = 30;

pub trait ChainGenerate {
    fn generate<R: Rng>(
        &self,
        selector: &Selector,
        rng: &mut R,
        min_words: usize,
        max_words: usize,
    ) -> Option<String>;
}

impl ChainGenerate for MarkovChain {
    fn generate<R: Rng>(
        &self,
        selector: &Selector,
        rng: &mut R,
        min_words: usize,
        max_words: usize,
    ) -> Option<String> {
        generate_sequence(selector, rng, min_words, max_words).map(|s| seq_to_text(s, &self.words))
    }
}

fn seq_to_text(seq: Vec<u32>, words: &IndexSet<String>) -> String {
    seq.into_iter()
        .filter_map(|word_idx| words.get_index(word_idx as usize).map(|w| w.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn generate_sequence<R: Rng>(
    selector: &Selector,
    rng: &mut R,
    min_words: usize,
    max_words: usize,
) -> Option<Vec<u32>> {
    let mut tries = 0;
    let mut generated: Vec<u32> = Vec::with_capacity(min_words as usize);
    let sources = selector.sources();
    let starting_edges: Vec<Vec<&ChainEntry>> = sources
        .iter()
        .map(|es| {
            es.entries
                .iter()
                .filter(|e| e.prefix.is_starting() && selector.filter_entry(e))
                .collect::<Vec<&ChainEntry>>()
        })
        .collect();

    while tries < MAX_TRIES {
        let mut edge_sources: HashSet<usize> = HashSet::with_capacity(sources.len());
        let mut next_edges: Vec<Vec<&ChainEntry>>;

        let (mut edge_source, mut edge) = pick_from_2d(&starting_edges, rng)?;
        loop {
            edge_sources.insert(edge_source);
            generated.extend_from_slice(&edge.prefix.word_idxs());
            if generated.len() > max_words {
                break;
            }
            if generated.len() >= min_words && edge.suffix.is_terminal() {
                generated.push(edge.suffix.word_idx());
                return Some(generated);
            }
            next_edges = sources
                .iter()
                .map(|es| {
                    es
                        .entries
                        .iter()
                        .filter(|e| {
                            e.prefix.word_idxs()[0] == edge.suffix.word_idx()
                                && selector.filter_entry(e)
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            match pick_from_2d(&next_edges, rng) {
                Some((e_source, e)) => {
                    edge_source = e_source;
                    edge = e;
                }
                None => break,
            }
        }
        generated.clear();
        tries += 1;
    }
    None
}

fn pick_from_2d<'a, T, S, R: Rng>(slices: &'a [S], rng: &mut R) -> Option<(usize, &'a T)>
where
    S: AsRef<[T]>,
{
    let total_len: usize = slices.iter().map(|s| s.as_ref().len()).sum();
    if total_len == 0 {
        None
    } else {
        let flat_idx = rng.gen_range(0, total_len);
        let mut slice_idx = 0;
        let mut elt_idx = 0;
        let mut traversed_len = 0;
        for s in slices {
            traversed_len += s.as_ref().len();
            if flat_idx < traversed_len {
                elt_idx = flat_idx - (traversed_len - s.as_ref().len());
                break;
            }
            slice_idx += 1;
        }
        let elt = &slices[slice_idx].as_ref()[elt_idx];
        Some((slice_idx, elt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChainAppend, ChainPrefix, ChainSuffix, Datestamp, TextSource};
    use indexmap::indexset;
    use rand::{rngs::SmallRng, SeedableRng};

    #[test]
    fn test_determined_generation() {
        let mut chain: MarkovChain = Default::default();
        chain.words.insert("сегодня".into());
        chain.words.insert("у".into());
        chain.words.insert("меня".into());
        chain.words.insert("депрессия".into());
        chain.words.insert("с".into());
        chain.words.insert("собаками".into());

        chain.sources.push(TextSource {
            names: indexset!["дана".into()],
            entries: vec![
                ChainEntry {
                    prefix: ChainPrefix::starting([0, 1]),
                    suffix: ChainSuffix::nonterminal(2),
                    datestamp: Datestamp {
                        year: 2070,
                        day: 360,
                    },
                },
                ChainEntry {
                    prefix: ChainPrefix::nonstarting([4, 5]),
                    suffix: ChainSuffix::terminal(6),
                    datestamp: Datestamp {
                        year: 2070,
                        day: 360,
                    },
                },
            ],
        });
        chain.sources.push(TextSource {
            names: indexset!["джилл".into()],
            entries: vec![ChainEntry {
                prefix: ChainPrefix::starting([2, 3]),
                suffix: ChainSuffix::nonterminal(4),
                datestamp: Datestamp {
                    year: 2070,
                    day: 360,
                },
            }],
        });

        let mut rng = SmallRng::from_seed([1; 16]);
        let selector = Selector::new(&chain, "джилл & дана", None).unwrap();
        let generated = chain.generate(&selector, &mut rng, 5, 6);
        assert_eq!(
            generated,
            Some("сегодня у меня депрессия с собаками".into())
        );
    }

    #[test]
    fn test_random_generation() {
        let mut chain = MarkovChain::new();
        chain.append_message_dump("tests/fixtures/messages.html");
        let mut rng = SmallRng::from_seed([1; 16]);
        let selector = Selector::new(&chain, "sota & denko", None).unwrap();
        let generated = chain.generate(&selector, &mut rng, 1, 3);
        assert_eq!(generated, Some("жасминовый чай (´･ω･`)".into()));
    }

    #[test]
    fn test_date_range_generation() {
        let mut chain = MarkovChain::new();
        chain.append_message_dump("tests/fixtures/messages.html");
        let mut rng = SmallRng::from_seed([1; 16]);

        let selector = Selector::new(
            &chain,
            "sota & denko",
            Some((
                Datestamp {
                    year: 2018,
                    day: 10,
                },
                Datestamp {
                    year: 2018,
                    day: 21,
                },
            )),
        )
        .unwrap();
        let generated = chain.generate(&selector, &mut rng, 2, 6);
        assert_eq!(generated, Some("жасминовый чай (´･ω･`)".into()));
    }
}
