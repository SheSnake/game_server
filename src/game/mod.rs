pub mod player;

extern crate rand;
use rand::{thread_rng, Rng};
use rand::seq::SliceRandom;

use player::Player;

pub mod majiang_model;
use majiang_model::Majiang;

pub mod majiang_state;
use majiang_state::GameState;

pub mod majiang_operation;
use majiang_operation::{MajiangOperation, Action};


pub struct Game {
    players: Vec<Player>,
    state: GameState,
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
            let card_id = self.state.next_card();
            self.state.print_state();
            // 2. judge cur_player can win or not
            let cur_player = self.state.cur_player();
            if let Some(win_op) = self.state.get_player_win_op(cur_player) {

                self.notify_operation(cur_player, &vec![win_op]);
            }
            
            if let Some(player_op) = self.mock_recv_player_pop(cur_player) {
                match player_op.op {
                    Action::POP => {
                        self.state.do_pop_card(cur_player, &player_op);
                        // 3. get operation of next player for this pop card
                        let mut other_ops: Vec<Option<Vec<MajiangOperation>>> =  vec![None, None, None, None];
                        for i in 1..4 {
                            let player = (cur_player + i) % 4;
                            if let Some(ops) = self.state.get_player_rsp_for_pop_card(player) {
                                self.notify_operation(player, &ops);
                                other_ops[player] = Some(ops);
                            }
                        }
                        self.state.print_state(); 
                        self.other_ops = other_ops;

                        if let Some(ops) = self.wait_for_player_operation(4) {
                        }
                        else {
                            self.state.next_player();
                            continue;
                        }
                    },
                    _ => ()
                }
            }

        }
    }

    fn notify_operation(&self, player: usize, ops: &Vec<MajiangOperation>) {
    }

    fn mock_recv_player_pop(&mut self, player: usize) -> Option<MajiangOperation> {
        let player_state = self.state.get_player_state(player);
        let cards = player_state.on_hand_card_id();
        let mut rng = thread_rng();
        if let Some(&ix) = cards.choose(&mut rng) {
            MajiangOperation::pop_card(ix)
        }
        else {
            None
        }
    }

    fn mock_recv_player_rsp(&mut self) {
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
    
    fn wait_for_player_operation(&mut self, timeout: u8) -> Option<Vec<MajiangOperation>> {
        self.recv_other_ops = vec![None, None, None, None];
        self.mock_recv_player_rsp();
        None
    }


}
