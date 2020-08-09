use redis::{Client, Commands, Connection, RedisResult};
use std::sync::{Arc, Mutex};

pub struct Redis {
    connection: Arc<Mutex<Connection>>,
}

pub struct ChatGameStorage {
    connection: Arc<Mutex<Connection>>,
    key_prefix: String,
}

impl Redis {
    pub fn new(redis_url: &str) -> RedisResult<Self> {
        let client = Client::open(redis_url)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(client.get_connection()?)),
        })
    }

    pub fn get_game_storage(&self, game: &str, chat_id: u64) -> ChatGameStorage {
        ChatGameStorage {
            connection: self.connection.clone(),
            key_prefix: format!("{}-{}", game, chat_id),
        }
    }
}

impl ChatGameStorage {
    pub fn incr_in_set(&mut self, set: &str, key: u64, by: i32) -> RedisResult<()> {
        self.connection
            .lock()
            .unwrap()
            .zincr(format!("{}-{}", self.key_prefix, set), key, by)
    }

    pub fn fetch_sorted_set(&mut self, set: &str) -> RedisResult<Vec<(u64, i32)>> {
        self.connection.lock().unwrap().zrevrangebyscore_withscores(
            format!("{}-{}", self.key_prefix, set),
            std::i32::MAX,
            std::i32::MIN,
        )
    }
}
