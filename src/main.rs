
pub mod game;
pub mod server_net;
use std::collections::HashMap;
use std::thread;
//use std::sync::mpsc::channel;
use server_net::message::*;
use server_net::kafka_client::*;
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
use tokio::stream::StreamExt;
use futures::*;
use rdkafka::producer::{ FutureProducer };


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

fn parse_authorized_user_id(msg: &Vec<u8>) -> i64 {
    let buf: &[u8] = &msg;
    let mut authorized_buf = [0u8; AUTHORIZED_INFO_SIZE];
    for i in 0..AUTHORIZED_INFO_SIZE {
        authorized_buf[i] = buf[i];
    }
    let authorized_user_id : i64 = i64::from_le_bytes(authorized_buf);
    return authorized_user_id;
}

#[tokio::main]
async fn main() {
    let (req_tx, mut req_rx)= channel::<Vec<u8>>(4096);
    let (mut rsp_tx, mut rsp_rx)= channel::<Vec<u8>>(4096);
    let req_tx_copy = req_tx.clone();
    let redis_addr = "redis://127.0.0.1:6379/".to_string();
    let redis_uri = redis_addr.clone();
    let worker_id = 1;
    let broker_addr = "127.0.0.1:9092".to_string();
    let group_id = format!("frontend_{}", worker_id);
    let topic_list_key = "topiclist:".to_string();

    let client_thread = thread::spawn(move || {
        let listen_addr = "0.0.0.0:8890".to_string();
        let writefd: Arc<Mutex<HashMap<i64, WriteHalf<TcpStream>>>> = Arc::new(Mutex::new(HashMap::new()));
        let writefd_copy = writefd.clone();
        let mut rt =  Runtime::new().unwrap();
        
        rt.spawn(async move {
            const AUTHORIZED_INFO_SIZE: usize = 8;
            loop {
                let msg = rsp_rx.recv().await.unwrap();
                let authorized_user_id = parse_authorized_user_id(&msg);
                let buf: &[u8] = &msg;
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
        rt.block_on(server_net::server_run(listen_addr, redis_uri, req_tx, writefd.clone()));
    });

    let listen_rsp_topics: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
    let topic_producer: Arc<Mutex<HashMap<String, FutureProducer>>> = Arc::new(Mutex::new(HashMap::new()));
    let topics_copy = listen_rsp_topics.clone();
    
    tokio::spawn(async move {
        let mut topic_mng = TopicMng::new(redis_addr, topic_list_key);
        loop {
            let data = req_rx.recv().await.unwrap();
            let buf: &[u8] = &data;
            let authorized_user_id = parse_authorized_user_id(&data);
            // if user is already in room, must can get the room_id and topic
            // otherwise, we only handle the room op
            if let Some(room_id) = topic_mng.get_user_room_id(&authorized_user_id).await {
                if let Some(topic) = topic_mng.get_room_topic(&room_id).await {
                    let rsp_topic = format!("{}_rsp", topic);
                    try_subscribe_topic(&broker_addr, &group_id, rsp_topic, rsp_tx.clone(), listen_rsp_topics.clone()).await;
                    produce(&broker_addr, &topic, &data, topic_producer.clone()).await;
                    continue;
                }
            }

            let header = bincode::deserialize::<Header> (&buf[AUTHORIZED_INFO_SIZE..AUTHORIZED_INFO_SIZE + HEADER_SIZE]).unwrap();
            println!("recv msg_type:{} from user:{}", header.msg_type, authorized_user_id);
            match unsafe { mem::transmute(header.msg_type) } {
                MsgType::RoomOp => {
                    let op = bincode::deserialize::<RoomManage> (&buf[AUTHORIZED_INFO_SIZE..]).unwrap();
                    let room_id: Vec<u8> = op.room_id.iter().cloned().collect();
                    let mut room_id = String::from_utf8(room_id).unwrap();
                    match unsafe { mem::transmute(op.op_type) } {
                        OpType::CreateRoom => {
                            if let Some(topic) = topic_mng.get_free_room_topic().await {
                                let rsp_topic = format!("{}_rsp", topic);
                                try_subscribe_topic(&broker_addr, &group_id, rsp_topic, rsp_tx.clone(), listen_rsp_topics.clone()).await;
                                produce(&broker_addr, &topic, &data, topic_producer.clone()).await;
                            }
                        },
                        _ => {
                            if let Some(topic) = topic_mng.get_room_topic(&room_id).await {
                                let rsp_topic = format!("{}_rsp", topic);
                                try_subscribe_topic(&broker_addr, &group_id, rsp_topic, rsp_tx.clone(), listen_rsp_topics.clone()).await;
                                produce(&broker_addr, &topic, &data, topic_producer.clone()).await;
                            }
                        }
                    }
                },
                _ => {
                    println!("cannot handle the msg_type[{}] for user_id[{}] not in room", header.msg_type, authorized_user_id);
                }
            }
        }
    });
    client_thread.join().unwrap();
}
