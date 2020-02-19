pub mod player;

extern crate rand;
use rand::{thread_rng, Rng};
use rand::seq::SliceRandom;

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
}

impl Game {
    pub fn new(players: Vec<Player>) -> Game {
        let num = players.len() as i64;
        Game {
            players: players,
            state: GameState::new(num),
            other_ops: Vec::new(),
            recv_other_ops: Vec::new(),
            cur_player_ops: None,
        }
    }

    pub fn init(&mut self) {
        self.state.init();
        self.state.deal_card();
        self.state.print_state();
    }

    pub fn start(&mut self) {
        while !self.state.over() {
            // 1. deal next card to cur player
            self.state.deal_next_card();
            self.state.print_state();
            // 2. figure out ops that cur_player can do (GANG or ZIMO)
            if let Some(ops) = self.state.get_cur_player_ops() {
                // notify cur_player if found
                let cur_player = self.state.cur_player();
                self.notify_operation(cur_player, &ops);
                self.cur_player_ops = Some(ops);
            } else {
                self.cur_player_ops = None;
            }

            while true {
                let cur_player = self.state.cur_player();
                let cur_player_op = self.wait_for_cur_player_op(8);
                match cur_player_op.op {
                    Action::POP => {
                        // 3. cur_player pop an card
                        self.state.do_pop_card(cur_player, &cur_player_op);
                        // 4. get rsp op of other player for this pop card
                        let mut other_ops: Vec<Option<Vec<MajiangOperation>>> =  vec![None, None, None, None];
                        for i in 1..4 {
                            let player = (cur_player + i) % 4;
                            // figure out other rsp op for this pop card
                            if let Some(ops) = self.state.get_player_rsp_for_pop_card(player) {
                                self.notify_operation(player, &ops);
                                other_ops[player] = Some(ops);
                            }
                        }
                        self.state.print_state(); 
                        self.other_ops = other_ops;

                        let recv = self.wait_for_other_player_op(4);
                        // if recv valid rsp op before timeout
                        if let Some((player, op)) = recv {
                            match op.op {
                                Action::HU => {
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
                    Action::ZI_MO => {
                        // only for first time of this iteration
                        self.state.execute_win_op(cur_player, &cur_player_op);
                        self.state.print_state();
                        break;
                    },
                    Action::GANG => {
                        // TODO
                    },
                    _ => () //  IMPOSSIBLE
                }
            }
        }
    }

    fn notify_operation(&self, player: usize, ops: &Vec<MajiangOperation>) {
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

    fn wait_for_cur_player_op(&mut self, timeout: u8) -> MajiangOperation {
        return self.mock_recv_cur_player_op();
    }
    
    fn wait_for_other_player_op(&mut self, timeout: u8) -> Option<(usize, MajiangOperation)> {
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
