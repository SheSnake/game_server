extern crate rand;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use super::majiang_model::Majiang;
use super::majiang_operation::{MajiangOperation , Action};

pub struct MajiangPlayerState {
    on_hand: HashMap<u8, u8>,
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

    pub fn new() -> MajiangPlayerState {
        MajiangPlayerState {
            on_hand: HashMap::new(),
            on_recv_now: None,
        }
    }

    pub fn on_hand_card_id(&self) -> Vec<u8> {
        let mut cards = Vec::new();
        for (&k, v) in self.on_hand.iter() {
            cards.push(k);
        }
        cards.sort();
        cards
    }
}

pub enum StateType {
    DEAL_CARD,
    WAIT_POP,
    WAIT_RESPONSE,
}

impl StateType {
    pub fn to_string(&self) -> String {
        match self {
            StateType::DEAL_CARD => "DEAL_CARD".to_string(),
            StateType::WAIT_POP => "WAIT_POP".to_string(),
            StateType::WAIT_RESPONSE => "WAIT_RESPONSE".to_string(),
        }
    }
}

pub struct GameState {
    num: i64,
    cur_step: i64,
    cur_state: StateType,
    cards: Vec<Majiang>,
    player_state: Vec<MajiangPlayerState>,
    hide_card: Vec<u8>,
    cur_player: usize,
    cur_card_ix: usize,
    cur_pop_card: u8,
}

impl GameState {
    pub fn new(num: i64) -> GameState {
        GameState {
            num: num,
            cur_step: 0,
            cur_state: StateType::DEAL_CARD,
            cards: Vec::new(),
            player_state: Vec::new(),
            hide_card: Vec::new(),
            cur_player: 0,
            cur_card_ix: 0,
            cur_pop_card: 0,
        }
    }

    pub fn init(&mut self) {
        for i in 1..10 {
            for j in 0..4 {
                self.cards.push(Majiang::Wan(i));
            }
        }
        for i in 1..10 {
            for j in 0..4 {
                self.cards.push(Majiang::Tiao(i));
            }
        }
        for i in 1..10 {
            for j in 0..4 {
                self.cards.push(Majiang::Bin(i));
            }
        }
        for j in 0..4 {
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
            let mut state = MajiangPlayerState::new();
            self.player_state.push(state);
            self.add_card(i, 13);
        }
    }

    pub fn next_card(&mut self) -> u8 {
        let card_id = self.hide_card[self.cur_card_ix];
        self.add_card(self.cur_player, 1);
        self.cur_step += 1;
        self.cur_state = StateType::WAIT_POP;
        card_id
    }

    pub fn next_player(&mut self) {
        self.cur_player += 1;
        self.cur_player %= 4;
    }

    pub fn cur_player(&self) -> usize {
        self.cur_player
    }

    pub fn get_player_op_by_card(&self, player: usize, card_id: u8) -> Option<Vec<MajiangOperation>> {
        None
    }

    pub fn get_player_win_op(&self, player: usize) -> Option<MajiangOperation> {
        None
    }

    pub fn do_pop_card(&mut self, player: usize, op: &MajiangOperation) {
        if self.player_state[player].has_card(op.target) {
            self.player_state[player].pop_card(op.target);
            self.cur_pop_card = op.target;
            self.cur_step += 1;
            self.cur_state = StateType::WAIT_RESPONSE;
        }
    }

    pub fn over(&self) -> bool {
        self.cur_card_ix == self.hide_card.len()
    }

    pub fn print_state(&self) {
        match self.cur_state {
            StateType::WAIT_POP => {
                println!("step:{} next_card_ix:{} state:{}", self.cur_step, self.cur_card_ix, self.cur_state.to_string());
            },
            StateType::WAIT_RESPONSE => {
                println!("step:{} next_card_ix:{} state:{} cur_pop_card:{}", self.cur_step, self.cur_card_ix, self.cur_state.to_string(), Majiang::format(&self.cards[self.cur_pop_card as usize]));
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
            if let Some(recv_now) = self.player_state[i].on_recv_now {
                let ix = recv_now as usize;
                content += &format!("on hand:{}", Majiang::format(&self.cards[ix]));
            }
            if i == self.cur_player {
                indicate = "*".to_string();
            }
            println!("{} player[{}]: {}\n",indicate, i, content);
        }
    }

    pub fn get_player_state(&self, player: usize) -> &MajiangPlayerState {
        &self.player_state[player]
    }
}


