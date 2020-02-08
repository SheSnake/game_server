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

pub struct MajiangOperation {
    pub op: Action,
    pub on_hand: Vec<u8>,
    pub target: u8,
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
}
