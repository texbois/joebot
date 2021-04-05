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

macro_rules! redis {
    ($s:ident) => {
        $s.connection.lock().unwrap()
    };
}

impl ChatGameStorage {
    pub fn add_gt_to_set(&mut self, set: &str, member: u64, score: i32) -> RedisResult<()> {
        let key = format!("{}-{}", self.key_prefix, set);
        let mut con = redis!(self);

        // ZADD GT is not available in Redis 4
        let old_score: Option<i32> = con.zscore(&key, member)?;
        if score > old_score.unwrap_or(0) {
            con.zadd(&key, member, score)?;
        }
        Ok(())
    }

    pub fn get_in_set(&mut self, set: &str, member: u64) -> RedisResult<i32> {
        let res: Option<i32> =
            redis!(self).zscore(format!("{}-{}", self.key_prefix, set), member)?;
        Ok(res.unwrap_or(0))
    }

    pub fn rem_from_set(&mut self, set: &str, member: u64) -> RedisResult<()> {
        redis!(self).zrem(format!("{}-{}", self.key_prefix, set), member)
    }

    pub fn incr_in_set(&mut self, set: &str, member: u64, by: i32) -> RedisResult<i32> {
        redis!(self).zincr(format!("{}-{}", self.key_prefix, set), member, by)
    }

    pub fn fetch_sorted_set(&mut self, set: &str) -> RedisResult<Vec<(u64, i32)>> {
        redis!(self).zrevrangebyscore_withscores(
            format!("{}-{}", self.key_prefix, set),
            std::i32::MAX,
            std::i32::MIN,
        )
    }
}
