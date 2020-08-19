use crate::{ChainEntry, Datestamp};

pub struct Selector {
    pub date_range: Option<(Datestamp, Datestamp)>,
}

impl Selector {
    pub fn filter_entry(&self, e: &ChainEntry) -> bool {
        if let Some((min_date, max_date)) = self.date_range {
            e.datestamp >= min_date && e.datestamp <= max_date
        } else {
            true
        }
    }
}
