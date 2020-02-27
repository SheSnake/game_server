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
    cur_round: i32,
    max_wait_second: u64,
    room_id: String,
    room_id_buf: [u8; 6],
}

impl Game {
    pub fn new(players: Vec<Player>, notifier: Sender<Vec<u8>>, receiver: Arc<Mutex<Receiver<Vec<u8>>>>, max_wait_second: u64, room_id: String) -> Game {
        let num = players.len();
        let mut buf = [0u8; 6];
        for (ix, &c) in room_id.clone().into_bytes().iter().enumerate() {
            buf[ix] = c;
        }
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
            cur_round: 1,
            room_id: room_id,
            room_id_buf: buf,
        };
    }

    pub fn init(&mut self) {
        self.state.init();
        self.state.deal_card();
        self.state.print_state();
    }

    pub fn reset(&mut self, next_banker: bool, players: Vec<Player>, cur_round: i32) {
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
        self.cur_round = cur_round;
    }

    pub fn get_win_user_id(&self) -> Option<i64> {
        if self.state.win_player < 4 {
            return Some(self.players[self.state.win_player].id);
        }
        return None;
    }

    pub fn get_cur_banker_pos(&self) -> usize {
        return self.cur_banker;
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
        // 0. notify new round card
        let begin_size = (mem::size_of::<Header>() + mem::size_of::<GameBasicInfo>() + 1 + 1 + 8 + 13) as i32;
        for i in 0..4 {
            let user_id = self.players[i as usize].id;
            let deal_begin_card = GameUpdate {
                header: Header {
                    msg_type: unsafe { mem::transmute(MsgType::GameUpdate) },
                    len: begin_size,
                },
                game_info: GameBasicInfo {
                    cur_game_step: self.state.cur_step,
                    cur_game_round: self.cur_round,
                    user_pos: i as u8,
                    user_id: user_id,
                    room_id: self.room_id_buf.clone(),
                },
                op_type: unsafe { mem::transmute(Action::DealBeginCard) },
                target: 0,
                provide_cards: self.state.player_state[i as usize].on_hand_card_id(),
            };
            let data: Vec<u8> = bincode::serialize::<GameUpdate>(&deal_begin_card).unwrap();
            self.send_data(&user_id, data).await;
        }

        while !self.state.over() {
            // 1. deal next card to cur player
            let on_hand_cards = self.state.player_state[self.state.cur_player()].on_hand_card_id();
            let next_card = self.state.deal_next_card();
            let cur_user_id = self.players[self.state.cur_player()].id;
            self.state.print_state();
            let data_len = (mem::size_of::<Header>() + mem::size_of::<GameBasicInfo>() + 1 + 1 + 8 + on_hand_cards.len()) as i32;
            let mut next_card_msg = GameUpdate {
                header: Header {
                    msg_type: unsafe { mem::transmute(MsgType::GameUpdate) },
                    len: data_len,
                },
                game_info: GameBasicInfo {
                    cur_game_step: self.state.cur_step,
                    cur_game_round: self.cur_round,
                    user_pos: self.state.cur_player() as u8,
                    user_id: cur_user_id,
                    room_id: self.room_id_buf.clone(),
                },
                op_type: unsafe { mem::transmute(Action::DealNextCard) },
                target: next_card,
                provide_cards: on_hand_cards,
            };
            let data: Vec<u8> = bincode::serialize::<GameUpdate>(&next_card_msg).unwrap();
            self.send_data(&cur_user_id, data).await;

            // notifier other user this step
            let data_len = (mem::size_of::<Header>() + mem::size_of::<GameBasicInfo>() + 1 + 1 + 8) as i32;
            next_card_msg.target = 0;
            next_card_msg.provide_cards.clear();
            next_card_msg.header.len = data_len;
            for i in 1..4 {
                let player = (self.state.cur_player() + i) % 4;
                let user_id = self.players[player].id;
                let data: Vec<u8> = bincode::serialize::<GameUpdate>(&next_card_msg).unwrap();
                self.send_data(&user_id, data).await;
            }
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
                let cur_player_op: MajiangOperation = self.wait_for_cur_player_op(self.max_wait_second * 30).await;
                match cur_player_op.op {
                    Action::Pop => {
                        // 3. cur_player pop an card
                        self.state.do_pop_card(cur_player, &cur_player_op);
                        // 4. get rsp op of other player for this pop card
                        let mut other_ops: Vec<Option<Vec<MajiangOperation>>> =  vec![None, None, None, None];
                        self.broadcast_game_update(&cur_player_op).await;
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

                        let recv = self.wait_for_other_player_op(self.max_wait_second * 30).await;
                        // if recv valid rsp op before timeout
                        if let Some((player, op)) = recv {
                            let mut over = false;
                            match op.op {
                                Action::Hu => {
                                    // if win over, over this game
                                    self.state.execute_win_op(player, &op);
                                    self.state.print_state();
                                    over = true;
                                },
                                _ => {
                                    // do op and change cur_player to this
                                    self.state.do_operation(player, &op);
                                    self.cur_player_ops = None;
                                    self.state.print_state(); 
                                }
                            }
                            self.broadcast_game_update(&op).await;
                            if over {
                                break;
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
                        self.broadcast_game_update(&cur_player_op).await;
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

    async fn notify_operation(&mut self, player: usize, ops: &Vec<MajiangOperation>) {
        let mut operations = Vec::new();
        let mut data_len = mem::size_of::<Header>() + 8;
        let user_id = self.players[player].id;
        for op in ops.iter() {
            let op_len = mem::size_of::<Header>() +  mem::size_of::<GameBasicInfo>() + 1 + 1 + 8 + op.on_hand.len();
            data_len += op_len;
            let op_data = GameOperation {
                header: Header {
                    msg_type: unsafe { mem::transmute(MsgType::GameOp) }, 
                    len: op_len as i32,
                },
                game_info: GameBasicInfo {
                    cur_game_step: self.state.cur_step,
                    cur_game_round: self.cur_round,
                    user_pos: player as u8,
                    user_id: user_id,
                    room_id: self.room_id_buf.clone(),
                },
                op_type: unsafe { mem::transmute(op.op) },
                target: op.target,
                provide_cards: op.on_hand.clone(),
            };
            operations.push(op_data);
        }
        let op_list = GameOperationPack {
            header: Header {
                msg_type: unsafe { mem::transmute(MsgType::GameOpPack) }, 
                len: data_len as i32,
            },
            operations: operations,
        };
        let data: Vec<u8> = bincode::serialize::<GameOperationPack>(&op_list).unwrap();
        self.send_data(&user_id, data).await;
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
        let data_len = mem::size_of::<Header>() +  mem::size_of::<GameBasicInfo>() + 1 + 1 + 8 + op.on_hand.len();
        let action_user_id = self.players[self.state.cur_player].id;
        let recv_user_id = self.players[player].id;
        let buff = [0u8; 6];
        let update = GameUpdate {
            header: Header {
                msg_type: unsafe { mem::transmute(MsgType::GameUpdate) }, 
                len: data_len as i32,
            },
            game_info: GameBasicInfo {
                cur_game_step: self.state.cur_step,
                cur_game_round: self.cur_round,
                user_pos: self.state.cur_player as u8,
                user_id: action_user_id,
                room_id: self.room_id_buf.clone(),
            },
            op_type: unsafe { mem::transmute(op.op) },
            target: op.target,
            provide_cards: op.on_hand.clone(),
        };
        let data: Vec<u8> = bincode::serialize::<GameUpdate>(&update).unwrap();
        self.send_data(&recv_user_id, data).await;
    }

    async fn broadcast_game_update(&mut self, op: &MajiangOperation) {
        for i in 0..4 {
            self.notify_game_update(i, op).await;
        }
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

    fn parse_authorized_info(&self, data: &Vec<u8>) -> i64 {
        const AUTHORIZED_INFO_SIZE: usize = 8;
        let mut authorized_buf = [0u8; AUTHORIZED_INFO_SIZE];
        for i in 0..AUTHORIZED_INFO_SIZE {
            authorized_buf[i] = data[i];
        }
        let authorized_user_id : i64 = i64::from_le_bytes(authorized_buf);
        return authorized_user_id;
    }

    async fn handle_query_state(&mut self, user_id: i64) {

    }

    fn get_user_pos(&self, user_id: i64) -> usize {
        for (ix, player) in self.players.iter().enumerate() {
            if player.id == user_id {
                return ix;
            }
        }
        return 0;
    }

    fn check_user_op_valid(&self, user_id: i64, msg: &GameOperation) -> bool {
        if msg.game_info.cur_game_round != self.cur_round {
            return false;
        }
        if msg.game_info.cur_game_step != self.state.cur_step {
            return false;
        }
        // TODO: more check
        return true;
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

    async fn read_game_operation_and_handle_query(&mut self) -> Option<(i64, GameOperation)> {
        if let Some(buf) = self.read_msg(20).await {
            let user_id = self.parse_authorized_info(&buf);
            const AUTHORIZED_INFO_SIZE: usize = 8;
            let header_size = mem::size_of::<Header>();
            let header = bincode::deserialize::<Header> (&buf[AUTHORIZED_INFO_SIZE..AUTHORIZED_INFO_SIZE + header_size]).unwrap();
            match unsafe { mem::transmute(header.msg_type) } {
                MsgType::GameOp => {
                    let op = bincode::deserialize::<GameOperation> (&buf[AUTHORIZED_INFO_SIZE..]).unwrap();
                    return Some((user_id, op));
                },
                MsgType::QueryGameState => {
                    self.handle_query_state(user_id);
                },
                _ => {
                }
            }
        }
        return None;
    }
    
    async fn wait_for_other_player_op(&mut self, timeout: u64) -> Option<(usize, MajiangOperation)> {
        struct TempOp {
            player: usize,
            op: MajiangOperation,
        }
        let mut possible_ops: Vec<TempOp> = Vec::new();
        for i in 0..4 {
            if let Some(player_ops) = &self.other_ops[i] {
                for op in player_ops.iter() {
                    possible_ops.push(TempOp {
                        player: i,
                        op: op.clone(),
                    });
                }
            }
        }
        possible_ops.sort_by(|a, b| a.op.op.priority().cmp(&b.op.op.priority()));

        if possible_ops.len() > 0 {
            let mut elapse = 0;
            while elapse < timeout {
                if let Some((user_id, op)) = self.read_game_operation_and_handle_query().await {
                    let pos = self.get_user_pos(user_id);
                    if self.check_user_op_valid(user_id, &op) {
                        loop {
                            let mut found = false;
                            let mut target: usize = 0;
                            for (ix, op) in possible_ops.iter().enumerate() {
                                if op.player == pos {
                                    target = ix;
                                    found = true;
                                    break;
                                }
                            }
                            if found {
                                possible_ops.remove(target);
                            } else {
                                break;
                            }
                        }
                        possible_ops.push(TempOp {
                            player: pos,
                            op: MajiangOperation {
                                op: unsafe { mem::transmute(op.op_type) },
                                on_hand: op.provide_cards,
                                target: op.target,
                        }});
                        possible_ops.sort_by(|a, b| a.op.op.priority().cmp(&b.op.op.priority()));
                        if possible_ops[0].player == pos {
                            println!("recv other choose");
                            break;
                        }
                    }
                }
                elapse += 20;
            }
            return Some((possible_ops[0].player, possible_ops[0].op.clone()));
        }
        return None;
    }
    
    async fn wait_for_cur_player_op(&mut self, timeout: u64) -> MajiangOperation {
        let mut elapse = 0;
        while elapse < timeout {
            if let Some((user_id, op)) = self.read_game_operation_and_handle_query().await {
                if self.check_user_op_valid(user_id, &op) {
                    return MajiangOperation {
                        op: unsafe { mem::transmute(op.op_type) },
                        on_hand: op.provide_cards,
                        target: op.target,
                    };
                }
            }
            elapse += 20;
        }
        return self.mock_recv_cur_player_op();
    }

}

pub struct StartGame {
    players: Vec<Player>,
    end_score: i32,
    cur_round: i32,
    game_notifier: Sender<Vec<u8>>,
    game_receiver: Arc<Mutex<Receiver<Vec<u8>>>>,
    max_wait_second: u64,
    room_id: String,
}

impl StartGame {
    pub fn new(room_id: String, players: Vec<i64>, notifier: Sender<Vec<u8>>, receiver: Receiver<Vec<u8>>) -> StartGame {
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
            room_id: room_id,
        };
    }

    fn get_player_score(&self) -> Vec<i32> {
        let mut score = Vec::new();
        for player in self.players.iter() {
            score.push(player.score);
        }
        return score;
    }

    pub fn over(&self) -> bool {
        for player in self.players.iter() {
            if player.score <= self.end_score {
                return true;
            }
        }
        return false;
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

    async fn broadcast_msg(&mut self, data: Vec<u8>) {
        let mut user_ids = Vec::new();
        for player in self.players.iter_mut() {
            user_ids.push(player.id);
        }
        for user_id in user_ids.iter(){
            self.send_data(user_id, data.clone()).await;
        }
    }

    pub async fn start(&mut self) {
        let mut round = Game::new(self.players.clone(), self.game_notifier.clone(), self.game_receiver.clone(), self.max_wait_second, self.room_id.clone());
        let mut next_banker = false;
        let round_update_len =  (mem::size_of::<Header>() + 1 + 4 + 1 + 8 + 8 + 4 * 4 + 8 + 4 * 4) as i32;
        while !self.over() {
            round.init();
            let msg = GameRoundUpdate {
                header: Header {
                    msg_type: unsafe { mem::transmute(MsgType::GameRoundUpdate) } ,
                    len: round_update_len,
                },
                round_info_type: unsafe{ mem::transmute(RoundInfoType::RoundStart) },
                cur_round: self.cur_round,
                cur_banker_pos: round.get_cur_banker_pos() as u8,
                cur_banker_user_id: self.players[round.get_cur_banker_pos()].id,
                user_cur_score: self.get_player_score(),
                user_score_change: vec![0, 0, 0, 0],
            };
            let data: Vec<u8> = bincode::serialize::<GameRoundUpdate>(&msg).unwrap();
            self.broadcast_msg(data).await;
            round.start().await;
            if let Some(_) = round.get_win_user_id() {
                next_banker = true
            }
            println!("round: {} over", self.cur_round);
            let mut score_change = Vec::new();
            for player in self.players.iter_mut() {
                let score = round.get_user_score(&player.id);
                player.score += score;
                score_change.push(score);
                println!("player: {} score:{}", player.id, player.score);
            }
            let msg = GameRoundUpdate {
                header: Header {
                    msg_type: unsafe { mem::transmute(MsgType::GameRoundUpdate) },
                    len: round_update_len,
                },
                round_info_type: unsafe{ mem::transmute(RoundInfoType::RoundOver) },
                cur_round: self.cur_round,
                cur_banker_pos: round.get_cur_banker_pos() as u8,
                cur_banker_user_id: self.players[round.get_cur_banker_pos()].id,
                user_cur_score: self.get_player_score(),
                user_score_change: score_change,
            };
            let data: Vec<u8> = bincode::serialize::<GameRoundUpdate>(&msg).unwrap();
            self.broadcast_msg(data).await;
            round.reset(next_banker, self.players.clone(), self.cur_round);
            self.cur_round += 1;
        }
    }
}

pub async fn start_game(room_id: String, players: Vec<i64>, notifier: Sender<Vec<u8>>, receiver: Receiver<Vec<u8>>) {
    let mut game = StartGame::new(room_id, players, notifier, receiver);
    game.start().await;

}
