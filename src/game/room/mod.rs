extern crate rand;
extern crate tokio;
use rand::{ thread_rng };
use rand::seq::SliceRandom;
use std::collections::HashMap;
use super::super::server_net::message::Code;
use tokio::sync::{ Mutex };
use tokio::sync::mpsc::{ Sender };
use std::sync::Arc;

pub enum RoomType {
    MAJIANG = 1,
}

pub struct GameRoom {
    pub room_id: String,
    pub room_type: u8,
    pub room_state: u8,
    pub max_player: usize,
    pub players: Vec<i64>, // player join in
    pub readys: HashMap<i64, bool>,
}

pub struct GameRoomMng {
    letters: Vec<char>,
    act_rooms: HashMap<String, GameRoom>,
    user_rooms: HashMap<i64, String>,
    start_game: HashMap<String, Sender<Vec<u8>>>,
    max_room_num: usize,
}

impl GameRoomMng {
    pub fn new(max_room_num: usize) -> GameRoomMng {
        return GameRoomMng {
            letters: "QAZWSXEDCRFVTGB1234567890".to_string().chars().collect(),
            act_rooms: HashMap::new(),
            user_rooms: HashMap::new(),
            start_game: HashMap::new(),
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

    pub fn create_room(&mut self, user_id: i64) -> (String, Code) {
        let err = "".to_string();
        if self.user_rooms.contains_key(&user_id) {
            let room_id = self.user_rooms.get(&user_id).unwrap().clone();
            return (room_id, Code::CreateFail);
        }
        if self.act_rooms.len() > self.max_room_num {
            return (err, Code::CreateFail);
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
            readys: HashMap::new(),
        };
        self.act_rooms.insert(room_id.clone(), room);
        self.user_rooms.insert(user_id, room_id.clone());
        return (room_id, Code::CreateOk);
    }

    pub fn join_room(&mut self, user_id: i64, room_id: &String) -> (String, Code) {
        let err = "".to_string();
        if self.user_rooms.contains_key(&user_id) {
            return (err, Code::AlreadyInRoom);
        }
        if let Some(room) = self.act_rooms.get_mut(room_id) {
            if room.players.len() > room.max_player {
                return (err, Code::RoomFull);
            }
            room.players.push(user_id);
            self.user_rooms.insert(user_id, room_id.clone());
            return (room_id.clone(), Code::JoinOk);
        }
        return (err, Code::RoomInexist);
    }

    pub fn ready_room(&mut self, user_id: i64, room_id: &String) -> (String, Code) {
        let err = "".to_string();
        if let Some(in_room) = self.user_rooms.get(&user_id) {
            if *in_room != *room_id {
                return (err, Code::WrongRoom);
            }
            if let Some(room) = self.act_rooms.get_mut(room_id) {
                room.readys.insert(user_id, true);
                return (room_id.clone(), Code::ReadyOk);
            }
            self.user_rooms.remove(&user_id);
            return (err, Code::RoomInexist);
        }
        return (err, Code::NotInRoom);
    }

    pub fn cancel_ready(&mut self, user_id: i64, room_id: &String) -> (String, Code) {
        let err = "".to_string();
        if let Some(in_room) = self.user_rooms.get(&user_id) {
            if *in_room != *room_id {
                return (err, Code::WrongRoom);
            }
            if let Some(room) = self.act_rooms.get_mut(room_id) {
                room.readys.remove(&user_id);
                return (room_id.clone(), Code::CancelReadyOk);
            }
            self.user_rooms.remove(&user_id);
            return (err, Code::RoomInexist);
        }
        return (err, Code::NotInRoom);
    }

    pub fn all_ready(&self, room_id: &String) -> Option<bool> {
        if let Some(room) = self.act_rooms.get(room_id) {
            return Some(room.readys.len() == room.max_player);
        }
        return None;
    }

    pub fn leave_room(&mut self, user_id: i64, room_id: &String) -> (String, Code) {
        let err = "".to_string();
        if let Some(in_room) = self.user_rooms.get(&user_id) {
            if *in_room != *room_id {
                return (err, Code::WrongRoom);
            }
            if let Some(room) = self.act_rooms.get_mut(room_id) {
                room.readys.remove(&user_id);
                for i in 0..room.players.len() {
                    if room.players[i] == user_id {
                        room.players.remove(i);
                        break;
                    }
                }
                if room.players.len() == 0 {
                    self.act_rooms.remove(room_id);
                    self.user_rooms.remove(&user_id);
                }
                return (room_id.clone(), Code::ReadyOk);
            }
            self.user_rooms.remove(&user_id);
            return (err, Code::RoomInexist);
        }
        return (err, Code::NotInRoom);
    }

    pub fn set_room_notifier(&mut self, room_id: &String, sender: Sender<Vec<u8>>) {
        if let Some(room) = self.act_rooms.get(room_id) {
            if room.readys.len() == room.max_player {
                self.start_game.insert(room_id.clone(), sender);
            }
        }
    }

    pub fn get_room_user_id(&self, room_id: &String) -> Option<Vec<i64>> {
        if let Some(room) = self.act_rooms.get(room_id) {
            return Some(room.players.clone());
        } else {
            return None;
        }
    }

    pub fn show_room_state(&self) {
        for (room_id, room) in self.act_rooms.iter() {
            println!("room:{} has user:{:?}, ready state:{:?}", room_id, room.players, room.readys);
        }
    }
}

