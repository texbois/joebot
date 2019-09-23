use redis::{Commands, Client, Connection, RedisResult};

pub struct Redis {
    connection: Connection
}

pub struct ChatGameStorage<'a> {
    connection: &'a mut Connection,
    key_prefix: String
}

impl Redis {
    pub fn new(redis_url: &str) -> Self {
        let client = Client::open(redis_url).unwrap();
        Self {
            connection: client.get_connection().unwrap()
        }
    }

    pub fn get_game_storage<'a>(&'a mut self, game: &str, chat_id: i64) -> ChatGameStorage<'a> {
        ChatGameStorage {
            connection: &mut self.connection,
            key_prefix: format!("{}-{}", game, chat_id)
        }
    }
}

impl<'a> ChatGameStorage<'a> {
    pub fn incr_in_set(&mut self, set: &str, key: &str, by: i32) -> RedisResult<()> {
        self.connection.zincr(
            format!("{}-{}", self.key_prefix, set), key, by)
    }

    pub fn fetch_sorted_set(&mut self, set: &str) -> RedisResult<Vec<(String, i32)>> {
        self.connection.zrevrangebyscore_withscores(
            format!("{}-{}", self.key_prefix, set), std::i32::MAX, std::i32::MIN)
    }
}
