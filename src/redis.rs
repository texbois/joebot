extern crate redis;
use redis::Commands;

pub struct Redis {
    connection: redis::Connection
}

impl Redis {
    pub fn new(chat_id: String) -> Self {
        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        Self {
            connection: client.get_connection().unwrap()
        }
    }

    pub fn add_result(&self, chat_id: u64, user_id: String, score: u8) {
        let _ : () = self.connection.zadd(user_id, chat_id, score).unwrap();   
    }

    pub fn get_stats(&self, chat_id: u64) -> Vec<(String, u32)> {
        return self.connection.zrevrangebyscore_withscores("ktey", 1000, -1000).unwrap();
    }
}
        

