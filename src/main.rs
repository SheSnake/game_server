
pub mod game;
pub mod server_net;
use std::collections::HashMap;
use std::thread;
//use std::sync::mpsc::channel;
use server_net::message::*;
use std::mem;
use game::room::GameRoomMng;
extern crate bincode;
extern crate tokio;
use tokio::sync::mpsc::channel;
use tokio::sync::Mutex;
use tokio::runtime::Runtime;
use std::sync::Arc;
use tokio::io::{ WriteHalf, AsyncWriteExt };
use tokio::net::{ TcpStream};

#[tokio::main]
async fn main() {
    let p1 = game::player::Player {
        id: 1,
        score: 0,
        fd: 0,
    };
    let p2 = game::player::Player {
        id: 2,
        score: 0,
        fd: 0,
    };
    let p3 = game::player::Player {
        id: 3,
        score: 0,
        fd: 0,
    };
    let p4 = game::player::Player {
        id: 4,
        score: 0,
        fd: 0,
    };


    let (req_tx, mut req_rx)= channel::<Vec<u8>>(4096);
    let (mut rsp_tx, mut rsp_rx)= channel::<Vec<u8>>(4096);
    let mut room_mng = GameRoomMng::new(3);
    let mut _round = game::Game::new(vec![p1, p2, p3, p4]);
    //round.init();
    //round.start();
    let t1 = thread::spawn(move || {
        //let mut server = Arc::new(Mutex::new(server_net::MultiThreadServer::new("0.0.0.0:8890".to_string(), req_tx)));
        let writefd: Arc<Mutex<HashMap<i64, WriteHalf<TcpStream>>>> = Arc::new(Mutex::new(HashMap::new()));
        let writefd_copy = writefd.clone();
        let mut rt =  Runtime::new().unwrap();
        rt.spawn(async move {
            loop {
                let buf: &[u8] = &rsp_rx.recv().await.unwrap();
                let op = bincode::deserialize::<RoomManageResult> (&buf[..]).unwrap();
                unsafe {
                    println!("open room id:{:?}", op.room_id);
                    let mut map = writefd_copy.lock().await;
                    if let Some(fd) = map.get_mut(&op.user_id) {
                        match fd.write(buf).await {
                            Ok(_) => {},
                            Err(_) => {},
                        }
                    }
                }
            }
        });
        rt.block_on(server_net::server_run("0.0.0.0:8890".to_string(), req_tx, writefd.clone()));
    });
    
    tokio::spawn(async move {
        let header_size = mem::size_of::<Header>();
        loop {
            let buf: &[u8] = &req_rx.recv().await.unwrap();
            const authorized_info_size: usize = 8;
            let mut authorized_buf = [0u8; authorized_info_size];
            for i in 0..authorized_info_size {
                authorized_buf[i] = buf[i];
            }
            let authorized_user_id : i64 = i64::from_le_bytes(authorized_buf);
            let header = bincode::deserialize::<Header> (&buf[authorized_info_size..header_size]).unwrap();
            match unsafe { mem::transmute(header.msg_type) } {
                MsgType::GameOp => {
                    match bincode::deserialize::<GameOperation> (&buf[authorized_info_size..]) {
                        Ok(game_op) => {
                            unsafe {
                                println!("recv provide:{:?} target:{}", game_op.provide_cards, game_op.target);
                            }
                        },
                        Err(err) => {
                            println!("parse message err: {:?}", err);
                        }
                    }
                },
                MsgType::RoomOp => {
                    let op = bincode::deserialize::<RoomManage> (&buf[authorized_info_size..]).unwrap();
                    let mut msg = RoomManageResult {
                        header: Header {
                            msg_type: 1,
                            len: 5 + 1 + 8 + 4 + 6,
                        },
                        op_type: op.op_type,
                        user_id: op.user_id,
                        code: 0,
                        room_id: vec![0; 6],
                    };
                    let room_id: Vec<u8> = op.room_id.iter().cloned().collect();
                    match unsafe { mem::transmute(op.op_type) } {
                        OpType::CreateRoom => {
                            let (room_id, code) = room_mng.create_room(op.user_id);
                            msg.room_id = room_id.clone().into_bytes();
                            msg.code = unsafe { mem::transmute(code) };
                            unsafe { println!("user:{} create room:{}", op.user_id.clone(), room_id) };
                        },
                        OpType::JoinRoom => {
                            let (err, code) = room_mng.join_room(op.user_id, String::from_utf8(room_id).unwrap());
                            msg.room_id = err.into_bytes();
                            msg.code = unsafe { mem::transmute(code) };
                        },
                        OpType::LeaveRoom => {
                            let (err, code) = room_mng.leave_room(op.user_id, String::from_utf8(room_id).unwrap());
                            msg.room_id = err.into_bytes();
                            msg.code = unsafe { mem::transmute(code) };
                        },
                        OpType::ReadyRoom => {
                            let (err, code) = room_mng.ready_room(op.user_id, String::from_utf8(room_id).unwrap());
                            msg.room_id = err.into_bytes();
                            msg.code = unsafe { mem::transmute(code) };
                        }
                    }
                    let data = bincode::serialize::<RoomManageResult>(&msg).unwrap();
                    match rsp_tx.send(data).await {
                        Ok(()) => {},
                        Err(_) => {
                            // TODO
                        }
                    }
                    room_mng.show_room_state();

                }
            }
        }
    });
    t1.join().unwrap();
}
