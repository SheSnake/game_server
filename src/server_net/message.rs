use serde::{Deserialize, Serialize};

pub enum MsgType {
    POP_CARD,
}

#[derive(Serialize, Deserialize)]
#[repr(packed(1))]
pub struct Header {
    pub msg_type: i8,
    pub len: i32,
}

#[derive(Serialize, Deserialize)]
#[repr(packed(1))]
pub struct Message {
    header: Header,
}
