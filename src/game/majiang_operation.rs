pub enum Action {
    POP,
    CHI,
    PENG,
    GANG,
    HU,
    ZI_MO,
    QIANG_JIN,
    QING_YI_SE,
    JIN_QUE,
    JIN_LONG,
}

use super::majiang_model::Majiang;

pub struct MajiangOperation {
    pub op: Action,
    pub on_hand: Vec<u8>,
    pub target: u8,
}

impl Action {
}


impl MajiangOperation {
    pub fn pop_card(card: u8) -> Option<MajiangOperation> {
        let mut op = MajiangOperation {
            op: Action::POP,
            on_hand: vec![card],
            target: card,
        };
        Some(op)
    }

    pub fn get_chi_op(avl_cards: &Vec<u8>, pop_card: u8) -> Option<Vec<MajiangOperation>> {
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
                    op: Action::CHI,
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
                op: Action::PENG,
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
                op: Action::GANG,
                on_hand: cards,
                target: pop_card,
            })
        }
        else {
            None
        }
    }

    pub fn get_win_op(avl_cards: &Vec<u8>, pop_card: u8) -> Option<MajiangOperation> {
    }

    pub fn to_string(&self, majiang_map: &Vec<Majiang>) -> String {
        match self.op {
            Action::CHI => {
                let ix_1 = self.on_hand[0] as usize;
                let ix_2 = self.on_hand[1] as usize;
                let chi = format!("CHI:{}||{}", Majiang::format(&majiang_map[ix_1]), Majiang::format(&majiang_map[ix_2])).to_string();
                chi
            },
            Action::PENG => {
                let ix_1 = self.on_hand[0] as usize;
                let peng = format!("PENG:{}||{}", Majiang::format(&majiang_map[ix_1]), Majiang::format(&majiang_map[ix_1])).to_string();
                peng
            },
            Action::GANG => {
                let ix_1 = self.on_hand[0] as usize;
                let gang = format!("GANG:{}||{}||{}", Majiang::format(&majiang_map[ix_1]), Majiang::format(&majiang_map[ix_1]), Majiang::format(&majiang_map[ix_1])).to_string();
                gang
            },
            _ => {
                "".to_string()
            }
        }
    }
}
