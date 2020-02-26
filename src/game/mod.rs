pub mod player;
pub mod room;

extern crate rand;
extern crate tokio;
use rand::{ thread_rng };
use rand::seq::SliceRandom;
use tokio::sync::{ Mutex };
use tokio::sync::mpsc::{ Sender, Receiver };
use std::time::Duration;
use tokio::time::timeout;
use std::sync::Arc;
use std::mem;
use super::server_net::message::*;

use player::Player;

pub mod majiang_model;

pub mod majiang_state;
use majiang_state::GameState;

pub mod majiang_operation;
use majiang_operation::{MajiangOperation, Action};


pub struct Game {
    players: Vec<Player>,
    state: GameState,
    cur_player_ops: Option<Vec<MajiangOperation>>,
    other_ops: Vec<Option<Vec<MajiangOperation>>>,
    recv_other_ops: Vec<Option<MajiangOperation>>,
    game_notifier: Sender<Vec<u8>>,
    game_receiver: Arc<Mutex<Receiver<Vec<u8>>>>,
    default_base_score: i32,
    base_score: i32,
    cur_banker: usize,
    max_wait_second: u64,
}

impl Game {
    pub fn new(players: Vec<Player>, notifier: Sender<Vec<u8>>, receiver: Arc<Mutex<Receiver<Vec<u8>>>>, max_wait_second: u64) -> Game {
        let num = players.len();
        return Game {
            players: players,
            state: GameState::new(num),
            other_ops: Vec::new(),
            recv_other_ops: Vec::new(),
            cur_player_ops: None,
            game_notifier: notifier,
            game_receiver: receiver,
            max_wait_second: max_wait_second,
            default_base_score: 5,
            base_score: 5,
            cur_banker: 0,
        };
    }

    pub fn init(&mut self) {
        self.state.init();
        self.state.deal_card();
        self.state.print_state();
    }

    pub fn reset(&mut self, next_banker: bool, players: Vec<Player>) {
        self.players = players;
        self.state = GameState::new(self.players.len());
        self.other_ops = Vec::new();
        self.cur_player_ops = None;
        if next_banker {
            self.base_score = self.default_base_score;
            self.cur_banker = (self.cur_banker + 1) % self.players.len();
        } else {
            self.base_score += 1;
        }
    }

    pub fn get_win_user_id(&self) -> Option<i64> {
        if self.state.win_player < 4 {
            return Some(self.players[self.state.win_player].id);
        }
        return None;
    }

    pub fn get_user_score(&self, user_id: &i64) -> i32 {
        for (ix, &player) in self.players.iter().enumerate() {
            if *user_id == player.id {
                if self.state.win_player >= 4 && ix == self.cur_banker {
                    return -self.base_score * 3;
                }
                if self.state.win_player >= 4 && ix != self.cur_banker {
                    return self.base_score;
                }
                if self.state.win_player < 4 && ix == self.state.win_player {
                    return self.base_score * 3;
                }
                return -self.base_score;
            }
        }
        return 0;
    }

    pub async fn start(&mut self) {
        while !self.state.over() {
            // 1. deal next card to cur player
            self.state.deal_next_card();
            self.state.print_state();
            // 2. figure out ops that cur_player can do (GANG or ZIMO)
            if let Some(ops) = self.state.get_cur_player_ops() {
                // notify cur_player if found
                let cur_player = self.state.cur_player();
                self.notify_operation(cur_player, &ops).await;
                self.cur_player_ops = Some(ops);
            } else {
                self.cur_player_ops = None;
            }

            loop {
                let cur_player = self.state.cur_player();
                let cur_player_op = self.wait_for_cur_player_op(self.max_wait_second * 1000).await;
                match cur_player_op.op {
                    Action::Pop => {
                        // 3. cur_player pop an card
                        self.state.do_pop_card(cur_player, &cur_player_op);
                        // 4. get rsp op of other player for this pop card
                        let mut other_ops: Vec<Option<Vec<MajiangOperation>>> =  vec![None, None, None, None];
                        for i in 1..4 {
                            let player = (cur_player + i) % 4;
                            // figure out other rsp op for this pop card
                            if let Some(ops) = self.state.get_player_rsp_for_pop_card(player) {
                                self.notify_operation(player, &ops).await;
                                other_ops[player] = Some(ops);
                            }
                        }
                        self.state.print_state(); 
                        self.other_ops = other_ops;

                        let recv = self.wait_for_other_player_op(4);
                        // if recv valid rsp op before timeout
                        if let Some((player, op)) = recv {
                            match op.op {
                                Action::Hu => {
                                    // if win over, over this game
                                    self.state.execute_win_op(player, &op);
                                    self.state.print_state();
                                    break;
                                },
                                _ => {
                                    // do op and change cur_player to this
                                    self.state.do_operation(player, &op);
                                    self.cur_player_ops = None;
                                    self.state.print_state(); 
                                }
                            }
                        } else {
                            self.state.next_player();
                            break;
                        }
                    },
                    Action::ZiMo => {
                        // only for first time of this iteration
                        self.state.execute_win_op(cur_player, &cur_player_op);
                        self.state.print_state();
                        break;
                    },
                    Action::Gang => {
                        // TODO
                    },
                    _ => () //  IMPOSSIBLE
                }
            }
        }
    }

    async fn notify_operation(&self, player: usize, ops: &Vec<MajiangOperation>) {
    }

    async fn send_data(&mut self, user_id: &i64, data: Vec<u8>) {
        let user_id = user_id.to_le_bytes();
        let user_id: Vec<u8> = user_id.iter().cloned().collect();
        let data = [user_id.clone(), data].concat();
        match self.game_notifier.send(data).await {
            Ok(()) => {},
            Err(_) => {
                // TODO
            }
        }
    }

    async fn notify_game_update(&mut self, player: usize, op: &MajiangOperation) {
        let provides_len = mem::size_of::<Header>() +  mem::size_of::<GameBasicInfo>() + 1 + 1 + 8 + op.on_hand.len();
        let action_user_id = self.players[self.state.cur_player].id;
        let recv_user_id = self.players[player].id;
        let update = GameUpdate {
            header: Header {
                msg_type: unsafe { mem::transmute(MsgType::GameUpdate) }, 
                len: provides_len as i32,
            },
            game_info: GameBasicInfo {
                cur_game_step: self.state.cur_step,
                player_id: self.state.cur_player as u8,
                user_id: action_user_id,
            },
            op_type: 1,
            target: op.target,
            provide_cards: op.on_hand.clone(),
        };
        let data: Vec<u8> = bincode::serialize::<GameUpdate>(&update).unwrap();
        self.send_data(&recv_user_id, data).await;
    }

    fn mock_recv_player_pop(&mut self, player: usize) -> Option<MajiangOperation> {
        let player_state = self.state.get_player_state(player);
        let cards = player_state.on_hand_card_id();
        if let Some(card) = MajiangOperation::select_advised_pop_card(&cards) {
            return MajiangOperation::pop_card(card);
        }
        let mut rng = thread_rng();
        if let Some(&ix) = cards.choose(&mut rng) {
            MajiangOperation::pop_card(ix)
        }
        else {
            None
        }
    }

    fn mock_recv_other_player_op(&mut self) {
        for i in 0..4 {
            if let Some(op) = &self.other_ops[i] {
               self.recv_other_ops[i] = Some(MajiangOperation {
                   op: op[0].op,
                   on_hand: op[0].on_hand.clone(),
                   target: op[0].target,
               });
            }
        }
    }

    fn mock_recv_cur_player_op(&mut self) -> MajiangOperation {
        let mut cur_priority = 0;
        let mut cur_ix = 0;
        if let Some(ops) = &self.cur_player_ops {
            for (ix, op) in ops.iter().enumerate() {
                if op.op.priority() > cur_priority {
                    cur_ix = ix;
                    cur_priority = op.op.priority();
                }
            }
        }
        if cur_priority != 0 {
            if let Some(ops) = &self.cur_player_ops {
                return MajiangOperation {
                    op: ops[cur_ix].op,
                    on_hand: ops[cur_ix].on_hand.clone(),
                    target: ops[cur_ix].target,
                };
            }
        }
        let op = self.mock_recv_player_pop(self.state.cur_player());
        return op.unwrap();
    }

    async fn read_msg(&mut self, max_wait: u64) -> Option<Vec<u8>> {
        {
            let mut receiver = self.game_receiver.lock().await;
            let process = receiver.recv();
            match timeout(Duration::from_millis(max_wait), process).await {
                Ok(res) => {
                    return Some(res.unwrap())
                },
                Err(_) => {
                    return None;
                }
            }
        }
    }

    async fn wait_for_cur_player_op(&mut self, timeout: u64) -> MajiangOperation {
        let mut elapse = 0;
        let mut recv = false;
        while elapse < timeout {
            if let Some(buf) = self.read_msg(20).await {
                let header_size = mem::size_of::<Header>();
                let header = bincode::deserialize::<Header> (&buf[..header_size]).unwrap();
                // ...
                println!("{}", header.msg_type);
                recv = true;
            }
            elapse += 20;
        }
        if !recv {
            println!("recv rsp timeout");
            return self.mock_recv_cur_player_op();
        } else {
            return self.mock_recv_cur_player_op();
        }
    }
    
    fn wait_for_other_player_op(&mut self, timeout: u64) -> Option<(usize, MajiangOperation)> {
        self.recv_other_ops = vec![None, None, None, None];
        self.mock_recv_other_player_op();
        let mut cur_player = 0;
        let mut cur_priority = 0;
        for (ix, op) in self.recv_other_ops.iter().enumerate() {
            if let Some(op) = op {
                if op.op.priority() > cur_priority {
                    cur_player = ix;
                    cur_priority = op.op.priority();
                }
            }
        }
        if cur_priority != 0 {
            if let Some(op) = &self.recv_other_ops[cur_player] {
                return Some((cur_player, MajiangOperation {
                    op: op.op,
                    on_hand: op.on_hand.clone(),
                    target: op.target,
                }));
            };
            None
        } else {
            None
        }
    }
}

pub struct StartGame {
    players: Vec<Player>,
    end_score: i32,
    cur_round: i32,
    game_notifier: Sender<Vec<u8>>,
    game_receiver: Arc<Mutex<Receiver<Vec<u8>>>>,
    max_wait_second: u64,
}

impl StartGame {
    pub fn new(players: Vec<i64>, notifier: Sender<Vec<u8>>, receiver: Receiver<Vec<u8>>) -> StartGame {
        let mut player_state = Vec::new();
        let start_score = 100;
        for (ix, &user_id) in players.iter().enumerate() {
            player_state.push({ Player {
                id: user_id,
                pos: ix,
                score: start_score,
            }});
        }
        return StartGame {
            players: player_state,
            end_score: 0,
            cur_round: 1,
            game_notifier: notifier,
            game_receiver: Arc::new(Mutex::new(receiver)),
            max_wait_second: 1,
        };
    }

    pub fn over(&self) -> bool {
        for player in self.players.iter() {
            if player.score <= self.end_score {
                return true;
            }
        }
        return false;
    }

    pub async fn start(&mut self) {
        let mut round = Game::new(self.players.clone(), self.game_notifier.clone(), self.game_receiver.clone(), self.max_wait_second);
        let mut next_banker = false;
        while !self.over() {
            round.init();
            round.start().await;
            self.cur_round += 1;
            if let Some(_) = round.get_win_user_id() {
                next_banker = true
            }
            println!("round: {} over", self.cur_round - 1);
            for player in self.players.iter_mut() {
                player.score += round.get_user_score(&player.id);
                println!("player: {} score:{}", player.id, player.score);
            }
            round.reset(next_banker, self.players.clone());

        }
    }
}

pub async fn start_game(players: Vec<i64>, notifier: Sender<Vec<u8>>, receiver: Receiver<Vec<u8>>) {
    let mut game = StartGame::new(players, notifier, receiver);
    game.start().await;

}
