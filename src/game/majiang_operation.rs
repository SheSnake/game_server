use std::mem;
use std::collections::HashMap;

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Action {
    Pop = 1,
    Chi = 2,
    Peng = 3,
    Gang = 4,
    Hu = 5,
    ZiMo = 6,
    QiangJin = 7,
    QingYiSe = 8,
    JinQue = 9,
    JinLong = 10,
    DealBeginCard = 11,
    DealNextCard = 12,
}

use super::majiang_model::Majiang;

pub struct MajiangOperation {
    pub op: Action,
    pub on_hand: Vec<u8>,
    pub target: u8,
}

impl Action {
    pub fn priority(&self) -> u8 {
        match self {
            Action::Chi => 1,
            Action::Peng => 2,
            Action::Gang => 3,
            Action::Hu => 4,
            Action::ZiMo => 5,
            _ => 0
        }
    }
}


impl MajiangOperation {
    pub fn pop_card(card: u8) -> Option<MajiangOperation> {
        let op = MajiangOperation {
            op: Action::Pop,
            on_hand: vec![card],
            target: card,
        };
        Some(op)
    }

    pub fn get_all_seq(cards: &Vec<u8>) -> Option<Vec<Vec<u8>>> {
        let mut seq_group: HashMap<Vec<u8>, u8> = HashMap::new();
        for &card in cards.iter() {
            let mut avl_card = Vec::new();
            for &avl in cards.iter() {
                if avl == card {
                    continue;
                }
                avl_card.push(avl);
            }
            if let Some(chi_ops) = MajiangOperation::get_chi_op(
                &avl_card,  card) {
                for op in chi_ops.iter() {
                    let mut seq = op.on_hand.clone();
                    seq.push(op.target);
                    seq.sort();
                    seq_group.insert(seq, 1);
                }
            }
            if let Some(peng_op) = MajiangOperation::get_peng_op(&avl_card, card) {
                let mut seq = peng_op.on_hand.clone();
                seq.push(peng_op.target);
                seq.sort();
                seq_group.insert(seq, 1);
            }
        }
        if seq_group.len() > 0 {
            let mut seqs = Vec::new();
            for k in seq_group.keys() {
                seqs.push(k.clone());
            }
            Some(seqs)
        }
        else {
            None
        }
    }

    pub fn all_in_seq(cards: &Vec<u8>) -> bool {
        if cards.len() == 0 {
            return true;
        }
        if cards.len() < 3 {
            return false;
        }
        if let Some(seqs) = MajiangOperation::get_all_seq(cards) {
            if cards.len() == 3 {
                return true;
            }
            let mut avl_cards = HashMap::new();
            for &avl in cards.iter() {
                avl_cards.insert(avl, 1);
            }
            for seq in seqs.iter() {
                for used in seq.iter() {
                    avl_cards.remove(used);
                }
                let mut left_cards = Vec::new();
                for (&k, _) in avl_cards.iter() {
                    left_cards.push(k);
                }
                if MajiangOperation::all_in_seq(&left_cards) {
                    return true;
                }
                for &used in seq.iter() {
                    avl_cards.insert(used, 1);
                }
            }
        }
        return false;
    }

    pub fn get_chi_op(avl_cards: &Vec<u8>, pop_card: u8) -> Option<Vec<MajiangOperation>> {
        if Majiang::is_bai_bang(pop_card) {
            return None;
        }
        let mut ops: Vec<MajiangOperation> = Vec::new();
        let to_found = [[1, 2], [-1, 1], [-1, -2]];
        for i in 0..3 {
            if (i == 0 || i == 1) && Majiang::end_card(pop_card) {
                continue;
            }
            if (i == 1 || i == 2) && Majiang::begin_card(pop_card) {
                continue;
            }
            if i == 0 && Majiang::end_card(pop_card + 4) {
                continue;
            }
            if i == 2 && Majiang::begin_card(pop_card - 4) {
                continue;
            }
            let mut found_cards: Vec<u8> = Vec::new();
            let to_found_ix_offset = &to_found[i];
            let pop_ix = (pop_card / 4) as i8;
            for offset in to_found_ix_offset.iter() {
                let to_found_ix = pop_ix + offset;
                for &card in avl_cards.iter() {
                    let card_ix = (card / 4) as i8;
                    if card_ix == to_found_ix {
                        found_cards.push(card);
                        break;
                    }
                }
            }
            if found_cards.len() == 2 {
                ops.push(MajiangOperation {
                    op: Action::Chi,
                    on_hand: found_cards,
                    target: pop_card,
                });
            }
        }
        if ops.len() > 0 {
            Some(ops)
        }
        else {
            None
        }
    }

    pub fn get_peng_op(avl_cards: &Vec<u8>, pop_card: u8) -> Option<MajiangOperation> {
        let mut cards: Vec<u8> = Vec::new(); 
        for &card in avl_cards.iter() {
            if (card / 4) == (pop_card / 4) {
                cards.push(card);
            }
            if cards.len() == 2 {
                break;
            }
        }
        if cards.len() == 2 {
            Some(MajiangOperation {
                op: Action::Peng,
                on_hand: cards,
                target: pop_card,
            })
        }
        else {
            None
        }
    }

    pub fn get_gang_op(avl_cards: &Vec<u8>, pop_card: u8) -> Option<MajiangOperation> {
        let mut cards: Vec<u8> = Vec::new(); 
        for &card in avl_cards.iter() {
            if (card / 4) == (pop_card / 4) {
                cards.push(card);
            }
            if cards.len() == 3 {
                break;
            }
        }
        if cards.len() == 3 {
            Some(MajiangOperation {
                op: Action::Gang,
                on_hand: cards,
                target: pop_card,
            })
        }
        else {
            None
        }
    }

    pub fn check_win(cards: &Vec<u8>) -> bool {
        let mut pair_group = HashMap::new();
        let mut on_hand = HashMap::new();
        for &card in cards.iter() {
            on_hand.insert(card, 1);
        }
        for (&k1, _) in on_hand.iter() {
            for (&k2, _) in on_hand.iter() {
                if k1 == k2 {
                    continue;
                }
                if k1 / 4 == k2 / 4 {
                    pair_group.insert(k1 / 4, (k1, k2));
                } 
            }
        }

        for (_k, (c1, c2)) in pair_group.iter() {
            on_hand.remove(c1);
            on_hand.remove(c2);
            let mut avl_cards = Vec::new();
            for (&card, _)  in on_hand.iter() {
                avl_cards.push(card);
            }
            if MajiangOperation::all_in_seq(&avl_cards) {
                return true;
            }
            on_hand.insert(*c1, 1);
            on_hand.insert(*c2, 1);
        }
        return false;
    }

    pub fn check_same_card(avl_cards: &Vec<u8>, target: u8) -> bool {
        for &card in avl_cards.iter() {
            if card == target {
                continue;
            }
            let ix = card / 4;
            if ix == target / 4 {
                return true;
            }
        }
        return false;
    }

    pub fn check_neighbor_card(avl_cards: &Vec<u8>, target: u8) -> bool {
        if Majiang::is_bai_bang(target) {
            return false;
        }
        let target_ix = target / 4;
        for &card in avl_cards.iter() {
            if card == target {
                continue;
            }
            let ix = card / 4;
            if Majiang::begin_card(target) {
                if ix == target_ix + 1 {
                    return true;
                }
            }
            else if Majiang::end_card(target) {
                if ix + 1 == target_ix {
                    return true;
                }
            }
            else if ix + 1 == target_ix || target_ix + 1 == ix {
                return true;
            }
        }
        return false;
    }

    pub fn select_advised_pop_card(avl_cards: &Vec<u8>) -> Option<u8> {
        for &card in avl_cards.iter() {
            if MajiangOperation::check_same_card(avl_cards, card) {
                continue;
            }
            if MajiangOperation::check_neighbor_card(avl_cards, card) {
                continue;
            }
            return Some(card);
        }
        return None;
    }

    pub fn to_string(&self, majiang_map: &Vec<Majiang>) -> String {
        match self.op {
            Action::Chi => {
                let ix_1 = self.on_hand[0] as usize;
                let ix_2 = self.on_hand[1] as usize;
                let chi = format!("CHI:{}||{}", Majiang::format(&majiang_map[ix_1]), Majiang::format(&majiang_map[ix_2])).to_string();
                chi
            },
            Action::Peng => {
                let ix_1 = self.on_hand[0] as usize;
                let peng = format!("PENG:{}||{}", Majiang::format(&majiang_map[ix_1]), Majiang::format(&majiang_map[ix_1])).to_string();
                peng
            },
            Action::Gang => {
                let ix_1 = self.on_hand[0] as usize;
                let gang = format!("GANG:{}||{}||{}", Majiang::format(&majiang_map[ix_1]), Majiang::format(&majiang_map[ix_1]), Majiang::format(&majiang_map[ix_1])).to_string();
                gang
            },
            _ => {
                "".to_string()
            }
        }
    }

    pub fn equal(&self, rhs: &MajiangOperation) -> bool {
        let op_type: u8 = unsafe{ mem::transmute(self.op) };
        if op_type != unsafe{ mem::transmute(rhs.op) } {
            return false;
        }
        if self.target != rhs.target {
            return false;
        }
        return true;
    }
}
