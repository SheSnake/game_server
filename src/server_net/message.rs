use serde::{Deserialize, Serialize};

pub enum MsgType {
    POP_CARD,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
#[repr(packed(1))]
pub struct Header {
    pub msg_type: i8,
    pub len: i32, // message length, include header and payload
}

/*
 * used for sync game time between client and server
 */

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
#[repr(packed(1))]
pub struct GameBasicInfo {
    pub cur_game_step: i64,
    pub player_id: u8,
    pub user_id: i64,
}


/*
 * game operation message
 * POP, HU, PENG, CHI, GANG, ZIMO and so on.
 */

#[derive(Serialize, Deserialize, Debug)]
#[repr(packed(1))]
pub struct GameOperation {
    pub header: Header,
    pub game_info: GameBasicInfo,
    pub op_type: i8,
    pub target: u8,
    pub provide_cards: Vec<u8>, // this is i64 + payload: [u8]
}
