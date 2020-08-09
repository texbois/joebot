mod append;
mod generate;

pub use append::ChainAppend;
pub use generate::ChainGenerate;

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

pub const NGRAM_CNT: usize = 2; // Use a bigram markov chain model

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Datestamp {
    pub year: i16,
    pub day: u16,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ChainPrefix([u32; NGRAM_CNT]); // indexes into MarkovChain.words

impl ChainPrefix {
    const fn new(word_idxs: [u32; NGRAM_CNT], starting: bool) -> Self {
        let word_idx0_31 = word_idxs[0] & ((1u32 << 31) - 1);
        Self([word_idx0_31 | (starting as u32) << 31, word_idxs[1]])
    }

    #[cfg(test)]
    const fn starting(word_idxs: [u32; NGRAM_CNT]) -> Self {
        Self::new(word_idxs, true)
    }

    #[cfg(test)]
    const fn nonstarting(word_idxs: [u32; NGRAM_CNT]) -> Self {
        Self::new(word_idxs, false)
    }

    const fn word_idxs(&self) -> [u32; NGRAM_CNT] {
        [self.0[0] & ((1u32 << 31) - 1), self.0[1]]
    }

    const fn is_starting(&self) -> bool {
        (self.0[0] & (1u32 << 31)) != 0
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainSuffix(u32);

impl ChainSuffix {
    const fn new(word_idx: u32, terminal: bool) -> Self {
        let word_idx_31 = word_idx & ((1u32 << 31) - 1);
        Self(word_idx_31 | (terminal as u32) << 31)
    }

    #[cfg(test)]
    const fn terminal(word_idx: u32) -> Self {
        Self::new(word_idx, true)
    }

    #[cfg(test)]
    const fn nonterminal(word_idx: u32) -> Self {
        Self::new(word_idx, false)
    }

    const fn word_idx(&self) -> u32 {
        self.0 & ((1u32 << 31) - 1)
    }

    const fn is_terminal(&self) -> bool {
        (self.0 & (1u32 << 31)) != 0
    }
}

impl std::fmt::Debug for ChainSuffix {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_terminal() {
            write!(f, "Terminal({})", self.word_idx())
        } else {
            write!(f, "NonTerminal({})", self.word_idx())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainEntry {
    pub prefix: ChainPrefix,
    pub suffix: ChainSuffix,
    pub datestamp: Datestamp,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct TextSource {
    pub names: IndexSet<String>,
    pub entries: Vec<ChainEntry>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct MarkovChain {
    pub words: IndexSet<String>,
    pub sources: Vec<TextSource>,
}

impl MarkovChain {
    pub fn new() -> Self {
        Default::default()
    }
}
