
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
            let header = bincode::deserialize::<Header> (&buf[0..header_size]).unwrap();
            unsafe {
                match mem::transmute(header.msg_type) {
                    MsgType::GameOp => {
                        println!("recv game op");
                        match bincode::deserialize::<GameOperation> (&buf[..]) {
                            Ok(game_op) => {
                                println!("recv provide:{:?} target:{}", game_op.provide_cards, game_op.target);
                            },
                            Err(err) => {
                                println!("parse message err: {:?}", err);
                            }
                        }
                    },
                    MsgType::RoomOp => {
                        let op = bincode::deserialize::<RoomManage> (&buf[..]).unwrap();
                        match mem::transmute(op.op_type) {
                            OpType::CreateRoom => {
                                if let Some(room_id) = room_mng.create_room(op.user_id) {
                                    let msg = RoomManageResult {
                                        header: Header {
                                            msg_type: 1,
                                            len: 5 + 1 + 8 + 4 + 6,
                                        },
                                        op_type: 1,
                                        user_id: op.user_id,
                                        code: 222,
                                        room_id: room_id.clone().into_bytes(),
                                    };
                                    let data = bincode::serialize::<RoomManageResult>(&msg).unwrap();
                                    match rsp_tx.send(data).await {
                                        Ok(()) => {},
                                        Err(_) => {
                                            // TODO
                                        }
                                    }
                                    println!("user:{} create room:{}", op.user_id, room_id);
                                }
                            },
                            OpType::JoinRoom => {
                            },
                            OpType::LeaveRoom => {
                            },
                            OpType::ReadyRoom => {
                            }
                        }
                    }
                }
            }
        }
    });
    t1.join().unwrap();
}
