extern crate tokio;
extern crate serde_derive;
extern crate serde;
extern crate serde_bytes;
extern crate bincode;
extern crate chrono;
extern crate redis;
use std::mem;
use std::sync::{Arc};
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{ Sender };
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{ WriteHalf, ReadHalf };
use tokio::prelude::*;
use tokio::time::timeout;
use std::collections::HashMap;
use redis::AsyncCommands;

pub mod message;
use message::*;

pub struct ClientSession {
    pub fd: ReadHalf<TcpStream>,
    pub buf: [u8; 4096],
    pub cur_head: usize,
    pub cur_tail: usize,
    pub cur_size: usize,
    pub total_size: usize,
}

impl ClientSession {
    pub fn new(fd: ReadHalf<TcpStream>) -> ClientSession {
        ClientSession {
            fd: fd,
            buf: [0; 4096],
            cur_head: 0,
            cur_tail: 0,
            cur_size: 0,
            total_size: 4096,
        }
    }

    pub fn rollbuf(&mut self) {
        if self.cur_head == 0 {
            return;
        }
        let mut ix: usize = 0;
        for i in self.cur_head..self.cur_tail {
            self.buf[ix] = self.buf[i];
            ix += 1;
        }
        self.cur_head = 0;
        self.cur_tail = ix;
    }

    pub fn reach_buf_end(&self) -> bool {
        return self.cur_tail == self.total_size;
    }
}

pub async fn get_userinfo_by_session_id(redis_uri: &String, session_id: String) -> Option<i64> {
    let mut sessions: HashMap<String, i64> = HashMap::new();
    let client = redis::Client::open(redis_uri.clone()).unwrap();
    let mut conn = client.get_async_connection().await.unwrap();
    match conn.get(session_id).await {
        Ok(v) => {
            println!("get auther_info:{}", &v);
            return Some(v);
        },
        Err(err) => {
            println!("redis get err: {}", err);
            return None;
        }
    }
}

pub async fn server_run(bind_addr: String, redis_addr: String, sender: Sender<Vec<u8>>, writefd_map: Arc<Mutex<HashMap<i64, WriteHalf<TcpStream>>>>) {
    let mut listener = TcpListener::bind(bind_addr).await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let mut sender = sender.clone();
        let writefd_map = writefd_map.clone();
        let (mut readfd,  mut writefd) = tokio::io::split(socket);
        let redis_uri = redis_addr.clone();
        tokio::spawn(async move {
            let mut authorized = false;
            let mut user_id: i64 = -1;
            const AUTHORIZED_SIZE: usize = 128;
            let mut buf = [0u8; AUTHORIZED_SIZE];
            let mut read_len = 0;
            println!("new connection");
            while read_len < AUTHORIZED_SIZE {
                let process = readfd.read(&mut buf[read_len..AUTHORIZED_SIZE]);
                match timeout(Duration::from_millis(5000), process).await {
                    Ok(res) => {
                        match res {
                            Ok(n) if n == 0 => {
                                println!("session close without authorized");
                                break;
                            },
                            Ok(n) => {
                                println!("buf {}, read {}, has read:{}", buf[0], n, read_len);
                                read_len += n;
                            },
                            Err(err) => {
                                println!("read err:{}", err);
                            }
                        }
                    },
                    Err(_) => {
                        println!("time out but no read");
                        break;
                    }
                }
            }

            if read_len == AUTHORIZED_SIZE {
                let session_id = String::from_utf8(buf.iter().cloned().collect()).unwrap();
                if let Some(user_info) = get_userinfo_by_session_id(&redis_uri, session_id).await {
                    user_id = user_info;
                    authorized = true;
                }
            }

            let mut msg = AuthenResult {
                header: Header {
                    msg_type: unsafe{ mem::transmute(MsgType::Authen) },
                    len: 0,
                },
                code: unsafe { mem::transmute(Code::AuthenOk)},
            };
            msg.header.len = msg.size() as i32;
            if !authorized {
                msg.code = unsafe { mem::transmute(Code::AuthenWrong)};
                let data = bincode::serialize::<AuthenResult> (&msg).unwrap();
                match writefd.write(&data).await {
                    Ok(_) => {},
                    Err(_) => {}
                }
                return;
            }
            let data = bincode::serialize::<AuthenResult> (&msg).unwrap();
            match writefd.write(&data).await {
                Ok(_) => {},
                Err(_) => {
                    return;
                }
            }

            let mut session = ClientSession::new(readfd);
            {
                let mut map = writefd_map.lock().await;
                map.insert(user_id, writefd);
            }

            println!("user_id:{} authorized", user_id);
            let authen_user_id = user_id;
            let user_id = user_id.to_le_bytes();
            let user_id: Vec<u8> = user_id.iter().cloned().collect();
            let msg = [user_id.clone(), data.iter().cloned().collect()].concat();
            match sender.send(msg).await {
                Ok(()) => {},
                Err(_) => {
                    // TODO
                }
            };

            loop {
                match session.fd.read(&mut session.buf[session.cur_tail..]).await {
                    Ok(n) if n == 0 => {
                        println!("session close");
                        break;
                    }
                    Ok(n) => {
                        session.cur_tail += n as usize;
                        session.cur_size += n as usize;
                        if session.reach_buf_end() {
                            session.rollbuf();
                        }
                        while session.cur_size >= HEADER_SIZE {
                            unsafe {
                                match bincode::deserialize::<Header> (&session.buf[session.cur_head..session.cur_head + HEADER_SIZE]) {
                                    Ok(header) => {
                                        if header.len > MAX_MSG_SIZE {
                                            {
                                                let mut map = writefd_map.lock().await;
                                                map.remove(&authen_user_id);
                                            }
                                            break;
                                        }
                                        println!("has read header, cur_head:{}, cur size:{}, need_len:{}", session.cur_head, session.cur_size, &header.len);
                                        if session.cur_size >= header.len as usize {
                                            let a: &[u8] = &session.buf[session.cur_head..session.cur_head + header.len as usize];
                                            let msg: Vec<u8> = a.iter().cloned().collect();
                                            let msg = [user_id.clone(), msg].concat();
                                            println!("send msg {:?}", msg);
                                            match sender.send(msg).await {
                                                Ok(()) => {},
                                                Err(_) => {
                                                    // TODO
                                                }
                                            };
                                            session.cur_size -= header.len as usize;
                                            session.cur_head += header.len as usize;
                                        }
                                        else {
                                            break;
                                        }
                                    },
                                    Err(err) => {
                                        println!("parse err: {:?}", err);
                                        break;
                                    },
                                }
                            }
                        }
                    },
                    Err(e) => {
                        println!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };
            }
        });
    }
}
