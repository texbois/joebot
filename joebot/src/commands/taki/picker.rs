use crate::{
    config::UserPenalties,
    messages::{Author, MessageDump},
};
use rand::{rngs::SmallRng, seq::SliceRandom};

pub struct SuspectPicker<'a> {
    suspects: Vec<Suspect<'a>>,
    user_penalties: &'a UserPenalties,
    game_idx: usize,
}

struct Suspect<'a> {
    author: &'a Author,
    texts: Vec<&'a str>,
    last_pick_game_idx: Option<usize>,
}

impl<'a> SuspectPicker<'a> {
    pub fn new(messages: &'a MessageDump, user_penalties: &'a UserPenalties) -> Self {
        let mut suspects = messages
            .authors
            .iter()
            .enumerate()
            .map(|(idx, author)| {
                let texts = messages
                    .texts
                    .iter()
                    .filter(|m| m.author_idx == idx)
                    .map(|m| m.text.as_str())
                    .collect::<Vec<_>>();

                Suspect {
                    author,
                    texts,
                    last_pick_game_idx: None,
                }
            })
            .collect::<Vec<Suspect>>();

        // Sort by number of texts descending
        suspects.sort_by(|a, b| b.texts.len().cmp(&a.texts.len()));

        user_penalties.verify_penalty_cap(suspects.len());

        Self {
            suspects,
            user_penalties,
            game_idx: 0,
        }
    }

    pub fn list_suspects(&self) -> impl Iterator<Item = &Author> {
        self.suspects.iter().map(|s| s.author)
    }

    pub fn random_suspect(
        &mut self,
        rng: &mut SmallRng,
        num_texts: usize,
    ) -> (&'a Author, Vec<&'a str>) {
        let num_suspects = self.suspects.len();
        let penalties = self.user_penalties;
        let last_game_idx = self.game_idx;

        let suspect = self
            .suspects
            .choose_weighted_mut(rng, |s| {
                suspect_weight(s, num_suspects, last_game_idx, penalties)
            })
            .unwrap();

        let sample_texts = suspect
            .texts
            .choose_multiple(rng, num_texts)
            .copied()
            .collect::<Vec<_>>();

        self.game_idx += 1;
        suspect.last_pick_game_idx = Some(self.game_idx);

        (suspect.author, sample_texts)
    }
}

fn suspect_weight(
    suspect: &Suspect,
    num_suspects: usize,
    last_game_idx: usize,
    penalties: &UserPenalties,
) -> usize {
    let name = &suspect.author.short_name;
    let penalty = penalties.by_short_name(name);
    let init_weight = num_suspects - penalty;

    match suspect.last_pick_game_idx {
        // Not included in the game
        _ if init_weight == 0 => 0,
        // We're just starting the game
        None => init_weight * 2,
        // Same suspect two times in a row
        Some(last_picked) if last_picked == last_game_idx => 0,
        // Init weight makes the next choice more random
        // (e.g. for 4 suspects the probabilities will be 0.389 0.334 0.277 0
        // vs 0.5 0.34 0.16 if we just take game_idx - last_picked)
        Some(last_picked) => init_weight + (last_game_idx - last_picked),
    }
}
