extern crate rand;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use super::majiang_model::Majiang;
use super::majiang_operation::{MajiangOperation , Action};

pub struct MajiangPlayerState {
    on_hand: HashMap<u8, u8>,
    group_cards: Option<Vec<Vec<u8>>>,
    pub on_recv_now: Option<u8>,
}

impl MajiangPlayerState {
    pub fn add_card(&mut self, cards: &[u8]) {
        for (_, &card) in cards.iter().enumerate() {
            self.on_hand.insert(card, 1);
        };
        if cards.len() == 1 {
            self.on_recv_now = Some(cards[0]);
        }
    }

    pub fn has_card(&mut self, card: u8) -> bool {
        self.on_hand.contains_key(&card)
    }

    pub fn pop_card(&mut self, card_id: u8) {
        self.on_hand.remove(&card_id);
        self.on_recv_now = None;
    }

    pub fn execute_op(&mut self, op: &MajiangOperation) {
        let mut group = Vec::new();
        match op.op {
            Action::Chi => {
                for &card in op.on_hand.iter() {
                    self.on_hand.remove(&card);
                    group.push(card); 
                }
            },
            Action::Peng => {
                for &card in op.on_hand.iter() {
                    self.on_hand.remove(&card);
                    group.push(card); 
                }
            },
            Action::Gang => {
                for &card in op.on_hand.iter() {
                    self.on_hand.remove(&card);
                    group.push(card); 
                }
            },
            _ => ()
        }
        group.push(op.target);
        group.sort();
        if let Some(group_cards) = &mut self.group_cards {
            group_cards.push(group);
        } else {
            self.group_cards = Some(vec![group]);
        }
    }

    pub fn new() -> MajiangPlayerState {
        MajiangPlayerState {
            on_hand: HashMap::new(),
            group_cards: None,
            on_recv_now: None,
        }
    }

    pub fn on_hand_card_id(&self) -> Vec<u8> {
        let mut cards = Vec::new();
        for (&k, _) in self.on_hand.iter() {
            cards.push(k);
        }
        cards.sort();
        cards
    }

    pub fn group_card_id(&self) -> Option<Vec<Vec<u8>>> {
        self.group_cards.clone()
    }

    pub fn get_win_op(&self) -> Option<MajiangOperation> {
        None
    }
}

pub enum StateType {
    DealCard,
    WaitPop,
    WaitResponse,
    WinOver,
    OutOfCard,
}

impl StateType {
    pub fn to_string(&self) -> String {
        match self {
            StateType::DealCard => "DEAL_CARD".to_string(),
            StateType::WaitPop => "WAIT_POP".to_string(),
            StateType::WaitResponse => "WAIT_RESPONSE".to_string(),
            StateType::WinOver => "WIN_OVER".to_string(),
            StateType::OutOfCard => "OUT_OF_CARD".to_string(),
        }
    }
}

pub struct GameState {
    pub cur_step: i64,
    pub cur_state: StateType,
    pub cards: Vec<Majiang>,
    pub player_state: Vec<MajiangPlayerState>,
    pub hide_card: Vec<u8>,
    pub win_player: usize,
    pub win_method: Action,
    pub cur_player: usize,
    pub cur_card_ix: usize,
    pub cur_pop_card: u8,
}

impl GameState {
    pub fn new(num: usize) -> GameState {
        GameState {
            cur_step: 0,
            cur_state: StateType::DealCard,
            cards: Vec::new(),
            player_state: Vec::new(),
            hide_card: Vec::new(),
            win_player: num + 1,
            win_method: Action::Hu,
            cur_player: 0,
            cur_card_ix: 0,
            cur_pop_card: 0,
        }
    }

    pub fn init(&mut self) {
        for i in 1..10 {
            for _j in 0..4 {
                self.cards.push(Majiang::Wan(i));
            }
        }
        for i in 1..10 {
            for _j in 0..4 {
                self.cards.push(Majiang::Tiao(i));
            }
        }
        for i in 1..10 {
            for _j in 0..4 {
                self.cards.push(Majiang::Bin(i));
            }
        }
        for _j in 0..4 {
            self.cards.push(Majiang::BaiBan);
        }
        for i in 0..112 {
            self.hide_card.push(i);
        }
        let mut rng = thread_rng();
        //rng.shuffle(&mut self.hide_card);
        self.hide_card.shuffle(&mut rng);
        println!("{:?}", self.hide_card);
    }

    fn add_card(&mut self, player: usize, num: usize) {
        if let Some(state) = self.player_state.get_mut(player as usize) {
            let begin = self.cur_card_ix;
            let end = self.cur_card_ix + num;
            state.add_card(&self.hide_card[begin..end]);
            self.cur_card_ix += num;
        }
    }

    pub fn deal_card(&mut self) {
        for i in 0..4 {
            let state = MajiangPlayerState::new();
            self.player_state.push(state);
            self.add_card(i, 13);
        }
    }

    pub fn deal_next_card(&mut self) -> u8 {
        let card_id = self.hide_card[self.cur_card_ix];
        self.add_card(self.cur_player, 1);
        self.cur_step += 1;
        self.cur_state = StateType::WaitPop;
        return card_id;
    }

    pub fn next_player(&mut self) {
        self.cur_player += 1;
        self.cur_player %= 4;
    }

    pub fn cur_player(&self) -> usize {
        self.cur_player
    }

    pub fn get_player_win_op(&self, player: usize) -> Option<MajiangOperation> {
        self.player_state[player].get_win_op();
        None
    }

    pub fn get_cur_player_ops(&self) -> Option<Vec<MajiangOperation>> {
        let mut ops = Vec::new();
        let avl_cards = self.player_state[self.cur_player].on_hand_card_id();
        if MajiangOperation::check_win(&avl_cards) {
            ops.push(MajiangOperation {
                op: Action::ZiMo,
                on_hand: avl_cards.clone(),
                target: self.cur_pop_card,
            })
        }
        if ops.len() > 0 {
            return Some(ops);
        }
        return None;
    }

    pub fn get_player_rsp_for_pop_card(&self, player: usize) -> Option<Vec<MajiangOperation>> {
        let mut ops: Vec<MajiangOperation> = Vec::new();
        let cards = self.player_state[player].on_hand_card_id();
        if player == (self.cur_player + 1) % 4 {
            if let Some(chi_ops) = MajiangOperation::get_chi_op(&cards, self.cur_pop_card)
            {
                ops.extend(chi_ops);
            }
        };
        if let Some(peng_op) = MajiangOperation::get_peng_op(&cards, self.cur_pop_card)
        {
            ops.push(peng_op);
        }
        if let Some(gang_op) = MajiangOperation::get_gang_op(&cards, self.cur_pop_card)
        {
            ops.push(gang_op);
        }
        let mut avl_cards = cards.clone();
        avl_cards.push(self.cur_pop_card);
        if MajiangOperation::check_win(&avl_cards) {
            ops.push(MajiangOperation {
                op: Action::Hu,
                on_hand: cards.clone(),
                target: self.cur_pop_card,
            });
        }

        if ops.len() > 0 {
            Some(ops)
        }
        else {
            None
        }
    }

    pub fn do_pop_card(&mut self, player: usize, op: &MajiangOperation) {
        if self.player_state[player].has_card(op.target) {
            self.player_state[player].pop_card(op.target);
            self.cur_pop_card = op.target;
            self.cur_step += 1;
            self.cur_state = StateType::WaitResponse;
        }
    }

    pub fn do_operation(&mut self, player: usize, op: &MajiangOperation) {
        self.cur_step += 1;
        self.cur_player = player;
        self.player_state[player].execute_op(op);
        self.cur_state = StateType::WaitPop;
    }

    pub fn execute_win_op(&mut self, player: usize, op: &MajiangOperation) {
        self.win_player = player;
        self.cur_player = player;
        self.cur_state = StateType::WinOver;
        self.cur_step += 1;
        self.win_method = op.op.clone();
    }

    pub fn over(&self) -> bool {
        if self.cur_card_ix == self.hide_card.len() {
            return true;
        }
        match self.cur_state {
            StateType::WinOver => {
                return true;
            }
            _ => {
                return false;
            }
        }
    }

    pub fn print_state(&self) {
        return;
        let mut in_wait_rsp = false;
        match self.cur_state {
            StateType::WaitPop => {
                println!("step:{} next_card_ix:{} state:{}", self.cur_step, self.cur_card_ix, self.cur_state.to_string());
            },
            StateType::WaitResponse => {
                in_wait_rsp = true;
                println!("step:{} next_card_ix:{} state:{} cur_pop_card:{}", self.cur_step, self.cur_card_ix, self.cur_state.to_string(), Majiang::format(&self.cards[self.cur_pop_card as usize]));
            },
            StateType::WinOver => {
                let mut win_str = "".to_string();
                match self.win_method {
                    Action::Hu => {
                        win_str = "HU".to_string();
                    },
                    Action::ZiMo => {
                        win_str = "ZIMO".to_string();
                    },
                    _ => (),
                }
                println!("step:{} win_method:{} state:{}", self.cur_step, win_str, self.cur_state.to_string());
            },
            _ => ()
        }
        for i in 0..4 {
            let cards = self.player_state[i].on_hand_card_id();
            let mut content = String::from("");
            let mut indicate = " ".to_string();
            for &v in cards.iter() {
                let ix = v as usize;
                let card = Majiang::format(&self.cards[ix]);
                content += &format!("{} ", card);
            }

            if let Some(groups) = self.player_state[i].group_card_id() {
                for group in groups.iter() {
                    let mut group_str =  "|".to_string();
                    for &card in group.iter() {
                        let ix = card as usize;
                        group_str += &format!("{} ", Majiang::format(&self.cards[ix]));
                    }
                    group_str += &"|".to_string();
                    content += &format!("{} ", group_str);
                }
            }
            if let Some(recv_now) = self.player_state[i].on_recv_now {
                let ix = recv_now as usize;
                content += &format!("on hand:{} ", Majiang::format(&self.cards[ix]));
            }
            if i == self.cur_player {
                indicate = "*".to_string();
            }
            else if i == self.win_player {
                indicate = "!".to_string();
            }
            else if in_wait_rsp {
                if let Some(ops) = self.get_player_rsp_for_pop_card(i) {
                    for op in ops.iter() {
                        content += &format!("{} ", MajiangOperation::to_string(&op, &self.cards));
                    }
                }
            }
            println!("{} player[{}]: {}\n",indicate, i, content);
        }
    }

    pub fn get_player_state(&self, player: usize) -> &MajiangPlayerState {
        &self.player_state[player]
    }
}


