use redis::{Commands, Client, Connection, RedisResult};

pub struct Redis {
    connection: Connection
}

pub struct ChatGameStorage<'a> {
    connection: &'a Connection,
    key_prefix: String
}

impl Redis {
    pub fn new(redis_url: &str) -> Self {
        let client = Client::open(redis_url).unwrap();
        Self {
            connection: client.get_connection().unwrap()
        }
    }

    pub fn get_game_storage<'a>(&'a self, game: &str, chat_id: u64) -> ChatGameStorage<'a> {
        ChatGameStorage {
            connection: &self.connection,
            key_prefix: format!("{}-{}", game, chat_id)
        }
    }
}
        
impl<'a> ChatGameStorage<'a> {
    pub fn incr_in_set(&self, set: &str, key: &str, by: i32) -> RedisResult<()> {
        self.connection.zincr(
            format!("{}-{}", self.key_prefix, set), key, by)
    }

    pub fn fetch_sorted_set(&self, set: &str) -> RedisResult<Vec<(String, i32)>> {
        self.connection.zrevrangebyscore_withscores(
            format!("{}-{}", self.key_prefix, set), std::i32::MAX, std::i32::MIN)
    }
}
