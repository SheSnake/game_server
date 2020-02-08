pub enum MessageType {
    NEW_RECV,
    NEW_POP,
}

pub struct message {
    msg_type: MessageType,
}
