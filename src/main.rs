
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
use tokio::sync::mpsc::{ channel, Sender };
use tokio::sync::{ Mutex };
use tokio::runtime::Runtime;
use std::sync::Arc;
use tokio::io::{ WriteHalf, AsyncWriteExt };
use tokio::net::{ TcpStream};

use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{CommitMode, Consumer, ConsumerContext, Rebalance};
use rdkafka::error::KafkaResult;
use rdkafka::message::{Headers, Message};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::util::get_rdkafka_version;
use tokio::stream::StreamExt;


struct CustomContext;

impl ClientContext for CustomContext {}

impl ConsumerContext for CustomContext {
    fn pre_rebalance(&self, rebalance: &Rebalance) {
        println!("Pre rebalance {:?}", rebalance);
    }

    fn post_rebalance(&self, rebalance: &Rebalance) {
        println!("Post rebalance {:?}", rebalance);
    }

    fn commit_callback(&self, result: KafkaResult<()>, _offsets: &TopicPartitionList) {
        println!("Committing offsets: {:?}", result);
    }
}

type LoggingConsumer = StreamConsumer<CustomContext>;

async fn consume_and_print(brokers: &str, group_id: &str, topics: &[&str]) {
    let context = CustomContext;

    let consumer: LoggingConsumer = ClientConfig::new()
        .set("group.id", group_id)
        .set("bootstrap.servers", brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        //.set("statistics.interval.ms", "30000")
        //.set("auto.offset.reset", "smallest")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create_with_context(context)
        .expect("Consumer creation failed");

    consumer.subscribe(&topics.to_vec()).expect("Can't subscribe to specified topics");

    // consumer.start() returns a stream. The stream can be used ot chain together expensive steps,
    // such as complex computations on a thread pool or asynchronous IO.
    println!("try start");
    let mut message_stream = consumer.start();

    while let Some(message) = message_stream.next().await {
        match message {
            Err(e) => println!("Kafka error: {}", e),
            Ok(m) => {
                let payload = m.payload().unwrap();
                let payload: Vec<u8> = payload.iter().cloned().collect();
                println!("key: '{:?}', payload: '{:?}', topic: {}, partition: {}, offset: {}, timestamp: {:?}",
                      m.key(), payload, m.topic(), m.partition(), m.offset(), m.timestamp());
                if let Some(headers) = m.headers() {
                    for i in 0..headers.count() {
                        let header = headers.get(i).unwrap();
                        println!("  Header {:#?}: {:?}", header.0, header.1);
                    }
                }
                consumer.commit_message(&m, CommitMode::Async).unwrap();
            }
        };
    }
}

use game::*;

async fn send_data(sender: &mut Sender<Vec<u8>>, user_id: &i64, data: Vec<u8>) {
    let user_id = user_id.to_le_bytes();
    let user_id: Vec<u8> = user_id.iter().cloned().collect();
    let data = [user_id.clone(), data].concat();
    match sender.send(data).await {
        Ok(()) => {},
        Err(_) => {
            // TODO
        }
    }
}

#[tokio::main]
async fn main() {
    let (req_tx, mut req_rx)= channel::<Vec<u8>>(4096);
    let (mut rsp_tx, mut rsp_rx)= channel::<Vec<u8>>(4096);
    let req_tx_copy = req_tx.clone();
    let redis_addr = "redis://127.0.0.1:6379/".to_string();
    let redis_uri = redis_addr.clone();
    let broker_port = "127.0.0.1:9092";
    static topic: &[&str] = &["test", "test2"];
    let group_id = "test-group";

    let client_thread = thread::spawn(move || {
        let listen_addr = "0.0.0.0:8890".to_string();
        let writefd: Arc<Mutex<HashMap<i64, WriteHalf<TcpStream>>>> = Arc::new(Mutex::new(HashMap::new()));
        let writefd_copy = writefd.clone();
        let mut rt =  Runtime::new().unwrap();
        
        rt.spawn(async move {
            const AUTHORIZED_INFO_SIZE: usize = 8;
            loop {
                let msg = rsp_rx.recv().await.unwrap();
                let buf: &[u8] = &msg;
                let mut authorized_buf = [0u8; AUTHORIZED_INFO_SIZE];
                for i in 0..AUTHORIZED_INFO_SIZE {
                    authorized_buf[i] = buf[i];
                }
                let authorized_user_id : i64 = i64::from_le_bytes(authorized_buf);
                {
                    let mut map = writefd_copy.lock().await;
                    if let Some(fd) = map.get_mut(&authorized_user_id) {
                        match fd.write(&buf[AUTHORIZED_INFO_SIZE..]).await {
                            Ok(_) => {},
                            Err(_) => {},
                        }
                    }
                }
            }
        });
        rt.spawn(consume_and_print(&broker_port, &group_id, topic));
        rt.block_on(server_net::server_run(listen_addr, redis_uri, req_tx, writefd.clone()));
    });
    
    tokio::spawn(async move {
        let mut room_mng = GameRoomMng::new(3, redis_addr);
        let header_size = mem::size_of::<Header>();
        loop {
            let msg = req_rx.recv().await.unwrap();
            let buf: &[u8] = &msg;
            let mut authorized_buf = [0u8; AUTHORIZED_INFO_SIZE];
            for i in 0..AUTHORIZED_INFO_SIZE {
                authorized_buf[i] = buf[i];
            }
            let authorized_user_id : i64 = i64::from_le_bytes(authorized_buf);
            let header = bincode::deserialize::<Header> (&buf[AUTHORIZED_INFO_SIZE..AUTHORIZED_INFO_SIZE + header_size]).unwrap();
            println!("recv msg_type:{} from user:{}", header.msg_type, authorized_user_id);
            match unsafe { mem::transmute(header.msg_type) } {
                MsgType::GameOp => {
                    match bincode::deserialize::<GameOperation> (&buf[AUTHORIZED_INFO_SIZE..]) {
                        Ok(game_op) => {
                            unsafe {
                                let room_id: Vec<u8> = game_op.game_info.room_id.iter().cloned().collect();
                                let room_id = String::from_utf8(room_id).unwrap();
                                if let Some(mut sender) = room_mng.get_room_notifier(&room_id) {
                                    println!("recv provide:{:?} target:{}", game_op.provide_cards, game_op.target);
                                    sender.send(msg).await;
                                }
                            }
                        },
                        Err(err) => {
                            println!("parse message err: {:?}", err);
                        }
                    }
                },
                MsgType::RoomOp => {
                    let op = bincode::deserialize::<RoomManage> (&buf[AUTHORIZED_INFO_SIZE..]).unwrap();
                    let mut msg = RoomManageResult {
                        header: Header::new(MsgType::RoomManageResult),
                        op_type: op.op_type,
                        user_id: op.user_id,
                        code: 0,
                        room_id: vec![0; 6],
                    };
                    let room_id: Vec<u8> = op.room_id.iter().cloned().collect();
                    let mut room_id = String::from_utf8(room_id).unwrap();
                    match unsafe { mem::transmute(op.op_type) } {
                        OpType::CreateRoom => {
                            let (created_room_id, code) = room_mng.create_room(op.user_id);
                            msg.room_id = created_room_id.clone().into_bytes();
                            msg.code = unsafe { mem::transmute(code) };
                            room_id = created_room_id;
                            unsafe { println!("user:{} create room:{}", op.user_id.clone(), room_id) };
                        },
                        OpType::JoinRoom => {
                            let (err, code) = room_mng.join_room(op.user_id, &room_id);
                            msg.room_id = err.into_bytes();
                            msg.code = unsafe { mem::transmute(code) };
                        },
                        OpType::LeaveRoom => {
                            let (err, code) = room_mng.leave_room(op.user_id, &room_id);
                            msg.room_id = err.into_bytes();
                            msg.code = unsafe { mem::transmute(code) };
                        },
                        OpType::ReadyRoom => {
                            let (err, code) = room_mng.ready_room(op.user_id, &room_id);
                            msg.room_id = err.into_bytes();
                            msg.code = unsafe { mem::transmute(code) };
                        },
                        OpType::CancelReady => {
                            let (err, code) = room_mng.cancel_ready(op.user_id, &room_id);
                            msg.room_id = err.into_bytes();
                            msg.code = unsafe { mem::transmute(code) };
                        },
                        _ => {}
                    }
                    msg.header.len = msg.size() as i32;
                    let data: Vec<u8> = bincode::serialize::<RoomManageResult>(&msg).unwrap();
                    println!("data len:{}", data.len());
                    send_data(&mut rsp_tx, &authorized_user_id, data).await;
                    room_mng.show_room_state();

                    if let Some(room_users) = room_mng.get_room_user_id(&room_id) {
                        let snapshot = room_mng.get_room_snapshot(&room_id).unwrap();
                        let data: Vec<u8> = bincode::serialize::<RoomSnapshot>(&snapshot).unwrap();
                        for user_id in room_users.iter() {
                            send_data(&mut rsp_tx, &user_id, data.clone()).await;
                        }
                        if let Some(all_ready) = room_mng.all_ready(&room_id) {
                            if all_ready  && !room_mng.room_has_start(&room_id) {
                                let mut update = RoomUpdate {
                                    header: Header::new(MsgType::RoomUpdate),
                                    op_type: unsafe { mem::transmute(OpType::StartRoom) },
                                    user_id: 0,
                                    room_id: room_id.clone().into_bytes(),
                                };
                                update.header.len = update.size() as i32;
                                let data: Vec<u8> = bincode::serialize::<RoomUpdate>(&update).unwrap();
                                for user_id in room_users.iter() {
                                    send_data(&mut rsp_tx, user_id, data.clone()).await;
                                }
                                let (game_msg_tx, game_msg_rx) = channel::<Vec<u8>>(4096);
                                room_mng.set_room_notifier(&room_id, game_msg_tx);
                                tokio::spawn(start_game(room_id.clone(), room_users.clone(), rsp_tx.clone(), game_msg_rx, req_tx_copy.clone()));
                            }
                        }
                    }
                },
                MsgType::GameOver => {
                    let over = bincode::deserialize::<GameOver> (&buf[AUTHORIZED_INFO_SIZE..]).unwrap();
                    let mut room_id = String::from_utf8(over.room_id.clone()).unwrap();
                    room_mng.room_game_over(&room_id);
                    if let Some(room_users) = room_mng.get_room_user_id(&room_id) {
                        let snapshot = room_mng.get_room_snapshot(&room_id).unwrap();
                        let data: Vec<u8> = bincode::serialize::<RoomSnapshot>(&snapshot).unwrap();
                        println!("room:{} over", room_id);
                        for user_id in room_users.iter() {
                            send_data(&mut rsp_tx, &user_id, data.clone()).await;
                        }
                    }
                },
                MsgType::Authen => {
                    if let Some(room_id) = room_mng.get_user_room_id(authorized_user_id) {
                        if let Some(mut sender) = room_mng.get_room_notifier(&room_id) {
                            let mut query = QueryGameSnapshot {
                                header: Header::new(MsgType::QueryGameState),
                                user_id: authorized_user_id,
                            };
                            query.header.len = query.size() as i32;
                            let data: Vec<u8> = bincode::serialize::<QueryGameSnapshot>(&query).unwrap();
                            send_data(&mut sender, &authorized_user_id, data).await;
                        } else {
                            let snapshot = room_mng.get_room_snapshot(&room_id).unwrap();
                            let data: Vec<u8> = bincode::serialize::<RoomSnapshot>(&snapshot).unwrap();
                            send_data(&mut rsp_tx, &authorized_user_id, data).await;
                        }
                    }
                }
                _ => {}
            }
        }
    });

    client_thread.join().unwrap();
}
