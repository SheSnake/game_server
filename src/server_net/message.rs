use serde::{Deserialize, Serialize};
use std::mem;

pub const MAX_MSG_SIZE: i32 = 1024;
pub const AUTHORIZED_INFO_SIZE: usize = 8;

#[repr(i8)]
pub enum MsgType {
    /*
     * room manage
     * */
    RoomOp = 10,
    RoomManageResult = 11,
    RoomSnapshot = 12,
    RoomUpdate = 13,

    /*
     * game op
     */
    GameOp = 20,
    GameOpPack = 21,
    GameUpdate = 22,
    GameRoundUpdate = 23,
    QueryGameState = 24,
    GameSnapshot = 25,
    GameOver = 26,

    Authen = 6,

}


#[repr(i8)]
pub enum OpType {
    CreateRoom = 1,
    JoinRoom = 2,
    LeaveRoom = 3,
    ReadyRoom = 4,
    StartRoom = 5,
    CancelReady = 6,
}

#[repr(i8)]
pub enum RoundInfoType {
    RoundStart = 0,
    RoundOver = 1,
}

#[repr(i32)]
pub enum Code {
    CreateOk = 1100,
    CreateFail = 1101,
    JoinOk = 1200,
    RoomFull = 1201,
    RoomInexist = 1202,
    AlreadyInRoom = 1203,
    ReadyOk = 1400,
    CancelReadyOk = 1600,
    WrongRoom = 1401,
    NotInRoom = 1402,
    AuthenOk = 6000,
    AuthenWrong = 6001,
    AuthenInExist = 6002,
}

#[repr(packed)]
#[derive(Copy, Clone)]
#[derive(Serialize, Deserialize)]
pub struct Header {
    pub msg_type: i8,
    pub len: i32, // message length, include header and payload
}

pub const HEADER_SIZE: usize = mem::size_of::<Header>();
impl Header {
    pub fn new(msg_type: MsgType) -> Header {
        return Header {
            msg_type: unsafe { mem::transmute(msg_type) },
            len: HEADER_SIZE as i32,
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
#[derive(Serialize, Deserialize)]
pub struct AuthenResult {
    pub header: Header,
    pub code: i32,
}

impl AuthenResult {
    pub fn size(&self) -> usize {
        return HEADER_SIZE + 4;
    }
}

/*
 * used for sync game time between client and server
 */

#[derive(Serialize, Deserialize, Copy, Clone)]
#[repr(packed)]
pub struct GameBasicInfo {
    pub cur_game_step: i64,
    pub cur_game_round: i32,
    pub user_pos: u8,
    pub user_id: i64,
    pub room_id: [u8; 6],
}
const GAME_INFO_SIZE: usize = mem::size_of::<GameBasicInfo>();

impl GameBasicInfo {
    pub fn new(round: i32, step: i64, user_pos: u8, user_id: i64, room_id: [u8; 6]) -> GameBasicInfo {
        return GameBasicInfo {
            cur_game_step: step,
            cur_game_round: round,
            user_pos: user_pos,
            user_id: user_id,
            room_id: room_id,
        }
    }
}


/*
 * game operation message
 * POP, HU, PENG, CHI, GANG, ZIMO and so on.
 */

#[derive(Serialize, Deserialize)]
#[repr(packed)]
pub struct GameOperation {
    pub header: Header,
    pub game_info: GameBasicInfo,
    pub op_type: i8,
    pub target: u8,
    pub provide_cards: Vec<u8>, // this is i64 + payload: [u8]
}

impl GameOperation {
    pub fn size(&self) -> usize {
        return HEADER_SIZE + GAME_INFO_SIZE + 2 + 8 + self.provide_cards.len();
    }
}

#[derive(Serialize, Deserialize)]
#[repr(packed)]
pub struct GameOperationPack {
    pub header: Header,
    pub operations: Vec<GameOperation>, // this is i64 + payload: [u8]
}
impl GameOperationPack {
    pub fn size(&self) -> usize {
        let mut len = HEADER_SIZE + 8;
        for op in self.operations.iter() {
            len += op.size();
        }
        return len;
    }
}

#[derive(Serialize, Deserialize)]
#[repr(packed)]
pub struct GameUpdate {
    pub header: Header,
    pub game_info: GameBasicInfo,
    pub op_type: i8,
    pub target: u8,
    pub provide_cards: Vec<u8>, // this is i64 + payload: [u8]
}

impl GameUpdate {
    pub fn size(&self) -> usize {
        return HEADER_SIZE + GAME_INFO_SIZE + 2 + 8 + self.provide_cards.len();
    }
}


/*
 * room operation message
 * crate join leave.
 */

#[derive(Deserialize)]
#[repr(packed)]
pub struct RoomManage {
    pub header: Header,
    pub op_type: i8,
    pub user_id: i64,
    pub room_id: [u8; 6], // 000000 for create
}

impl RoomManage {
    pub fn size(&self) -> usize {
        return HEADER_SIZE + 1 + 8 + 6;
    }
}

#[derive(Serialize, Deserialize)]
#[repr(packed)]
pub struct RoomManageResult {
    pub header: Header,
    pub op_type: i8,
    pub user_id: i64,
    pub code: i32,
    pub room_id: Vec<u8>, // 000000 for create
}

impl RoomManageResult {
    pub fn size(&self) -> usize {
        return HEADER_SIZE + 1 + 8 + 4 + 8 + self.room_id.len();
    }
}

#[derive(Serialize, Deserialize)]
#[repr(packed)]
pub struct RoomUpdate {
    pub header: Header,
    pub op_type: i8,
    pub user_id: i64,
    pub room_id: Vec<u8>, // 000000 for create
}

impl RoomUpdate {
    pub fn size(&self) -> usize {
        return HEADER_SIZE + 1 + 8 + 8 + self.room_id.len();
    }
}

#[derive(Serialize, Deserialize)]
#[repr(packed)]
pub struct RoomSnapshot {
    pub header: Header,
    pub user_pos: Vec<i64>,
    pub user_ready_status: Vec<u8>,
    pub room_id: Vec<u8>, // 000000 for create
}

impl RoomSnapshot {
    pub fn size(&self) -> usize {
        return HEADER_SIZE + 8 + 8 * self.user_pos.len() + 8 + 1 * self.user_ready_status.len() + 8 + 1 * self.room_id.len();
    }
}

#[derive(Serialize, Deserialize)]
#[repr(packed)]
pub struct GameRoundUpdate {
    pub header: Header,
    pub round_info_type: i8,
    pub cur_round: i32,
    pub cur_banker_pos: u8,
    pub cur_banker_user_id: i64,
    pub user_cur_score: Vec<i32>,
    pub user_score_change: Vec<i32>,
}

impl GameRoundUpdate {
    pub fn size(&self) -> usize {
        return HEADER_SIZE + 1 + 4 + 1 + 8 + 8 + self.user_cur_score.len() * 4 + 8 + self.user_score_change.len() * 4;
    }
}

#[derive(Serialize, Deserialize)]
#[repr(packed)]
pub struct QueryGameSnapshot {
    pub header: Header,
    pub user_id: i64,
}

impl QueryGameSnapshot {
    pub fn size(&self) -> usize {
        return HEADER_SIZE + 8;
    }
}

#[derive(Serialize, Deserialize)]
#[repr(packed)]
pub struct GameSnapshot {
    pub header: Header,
    pub game_info: GameBasicInfo,
    pub user_id: i64,
    pub user_on_hand: Vec<u8>,
    pub on_game_user_id: Vec<i64>, // user_id of 4 pos,
    pub on_game_group_cards: Vec<Vec<Vec<u8>>>,
}

impl GameSnapshot {
    pub fn size(&self) -> usize {
        let mut len = HEADER_SIZE + GAME_INFO_SIZE;
        len += 8;
        len += 8 + self.user_on_hand.len() * 1;
        len += 8 + self.on_game_user_id.len() * 8;
        len += 8;
        for user_group in self.on_game_group_cards.iter() {
            len += 8;
            for group in user_group.iter() {
                len += 8 + group.len();
            }
        }
        return len;
    }
}

#[derive(Serialize, Deserialize)]
#[repr(packed)]
pub struct GameOver {
    pub header: Header,
    pub room_id: Vec<u8>,
    pub cur_round: i32,
    pub user_cur_score: Vec<i32>,
}

impl GameOver {
    pub fn size(&self) -> usize {
        return HEADER_SIZE + 8 + 1 * self.room_id.len() + 4 + 8 + 4 * self.user_cur_score.len();
    }
}
