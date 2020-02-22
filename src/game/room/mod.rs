extern crate rand;
use rand::{ thread_rng };
use rand::seq::SliceRandom;
use std::collections::HashMap;

pub enum RoomType {
    MAJIANG = 1,
}

pub struct GameRoom {
    pub room_id: String,
    pub room_type: u8,
    pub room_state: u8,
    pub max_player: i8,
    pub players: Vec<i64>, // player join in
}

pub struct GameRoomMng {
    letters: Vec<char>,
    act_rooms: HashMap<String, GameRoom>,
    max_room_num: usize,
}

impl GameRoomMng {
    pub fn new(max_room_num: usize) -> GameRoomMng {
        return GameRoomMng {
            letters: "QAZWSXEDCRFVTGB1234567890".to_string().chars().collect(),
            act_rooms: HashMap::new(),
            max_room_num: max_room_num,
        };
    }

    fn random_room_id(&self) -> String {
        let mut room_id = String::from("");
        let mut rng = thread_rng();
        for _i in 0..6 {
            if let Some(&c) = self.letters.choose(&mut rng) {
                room_id.push(c as char);
            }
        }
        return room_id;
    }

    pub fn create_room(&mut self, user_id: i64) -> Option<String> {
        if self.act_rooms.len() > self.max_room_num {
            return None;
        };
        let mut room_id = self.random_room_id();
        while self.act_rooms.contains_key(&room_id) {
            room_id = self.random_room_id();
        }
        let room = GameRoom {
            room_id: room_id.clone(),
            room_type: 1,
            room_state: 0,
            max_player: 4,
            players: vec![user_id],
        };
        self.act_rooms.insert(room_id.clone(), room);
        return Some(room_id);
    }
}
