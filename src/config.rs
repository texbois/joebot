use regex::Regex;
use serde::{
    de::{Error, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

#[derive(Deserialize)]
pub struct Config {
    pub channel_id: u64,
    pub user_matcher: UserMatcher,
}

pub struct UserMatcher(HashMap<String, Regex>);

impl UserMatcher {
    pub fn short_names(&self) -> HashSet<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }

    pub fn matches_short_name(&self, input: &str, short_name: &str) -> bool {
        self.0[short_name].is_match(input)
    }
}

impl<'de> Deserialize<'de> for UserMatcher {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(UserMatcherVisitor)
    }
}

struct UserMatcherVisitor;

impl<'de> Visitor<'de> for UserMatcherVisitor {
    type Value = UserMatcher;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("regex map")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut map = HashMap::with_capacity(access.size_hint().unwrap_or(0));

        while let Some(k) = access.next_key()? {
            if let Ok(v) = access.next_value::<&str>() {
                map.insert(k, Regex::new(v).map_err(M::Error::custom)?);
            }
        }

        Ok(UserMatcher(map))
    }
}
