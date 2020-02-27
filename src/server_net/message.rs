use serde::{Deserialize, Serialize};

#[repr(i8)]
pub enum MsgType {
    /*
     * room manage
     * */
    RoomOp = 1,

    /*
     * game op
     */
    GameOp = 0,

    GameOpPack = 3,

    GameUpdate = 2,

    GameRoundUpdate = 4,

    QueryGameState = 5,
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
}

#[repr(packed)]
#[derive(Copy, Clone)]
#[derive(Serialize, Deserialize)]
pub struct Header {
    pub msg_type: i8,
    pub len: i32, // message length, include header and payload
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

#[derive(Serialize, Deserialize)]
#[repr(packed)]
pub struct GameOperationPack {
    pub header: Header,
    pub operations: Vec<GameOperation>, // this is i64 + payload: [u8]
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

#[derive(Serialize, Deserialize)]
#[repr(packed)]
pub struct RoomManageResult {
    pub header: Header,
    pub op_type: i8,
    pub user_id: i64,
    pub code: i32,
    pub room_id: Vec<u8>, // 000000 for create
}

#[derive(Serialize, Deserialize)]
#[repr(packed)]
pub struct RoomUpdate {
    pub header: Header,
    pub op_type: i8,
    pub user_id: i64,
    pub room_id: Vec<u8>, // 000000 for create
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
