extern crate tokio;
extern crate serde_derive;
extern crate serde;
extern crate serde_bytes;
extern crate bincode;
use std::mem;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::io::Read;
use std::thread;
use std::sync::{Mutex, Arc, mpsc};
use std::sync::mpsc::{ Sender, Receiver };
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;
use tokio::stream::Stream;
use tokio::prelude::*;

pub mod message;
use message::*;

pub struct ClientSession {
    pub fd: TcpStream,
    pub buf: [u8; 4096],
    pub cur_head: usize,
    pub cur_tail: usize,
    pub cur_size: usize,
    pub total_size: usize,
}

impl ClientSession {
    pub fn new(fd: TcpStream) -> ClientSession {
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

pub struct MultiThreadServer {
    bind_addr: String, 
    state: i8,
}

pub struct SessionJob {
}

pub struct ClientSessionManager {
    sessions: Arc<Mutex<Vec<ClientSession>>>,
    rx: Receiver<SessionJob>,
    tx: Sender<SessionJob>,
}

impl ClientSessionManager {
    pub fn new(sessions: Arc<Mutex<Vec<ClientSession>>>, tx: Sender<SessionJob>, rx: Receiver<SessionJob>) -> ClientSessionManager {
        ClientSessionManager {
            sessions: sessions,
            rx: rx,
            tx: tx,
        }
    }

    pub fn start(&mut self) {
    }
}

impl MultiThreadServer {
    pub fn new(addr_v4: String) -> MultiThreadServer {
        return MultiThreadServer {
            bind_addr: addr_v4,
            state: 0,
        }
    }

    async fn _run(&mut self) {
        println!("1233");
        let mut listener = TcpListener::bind(&self.bind_addr).await.unwrap();
        //let (tx1, rx1) = mpsc::channel();
        //let (tx2, rx2) = mpsc::channel();
        let session_mng: Arc<Mutex<Vec<ClientSession>>> = Arc::new(Mutex::new(Vec::new()));
        let mng = Arc::clone(&session_mng);
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let mut session = ClientSession::new(socket);
                loop {
                    println!("try read");
                    match session.fd.read(&mut session.buf[session.cur_tail..]).await {
                        Ok(n) if n == 0 => {
                            println!("session close");
                            return;
                        }
                        Ok(n) => {
                            session.cur_tail += n as usize;
                            session.cur_size += n as usize;
                            if session.reach_buf_end() {
                                println!("reach buf end!!");
                                session.rollbuf();
                            }
                            let header_size = mem::size_of::<Header>();
                            // for each read, handle all the finished recv meesage
                            while session.cur_size >= header_size {
                                match bincode::deserialize::<Header> (&session.buf[session.cur_head..session.cur_head + header_size]) {
                                    Ok(header) => {
                                        println!("has read header, cur_head:{}, cur size:{}, need_len:{}", session.cur_head, session.cur_size, &header.len);
                                        if session.cur_size >= header.len as usize {
                                            match header.msg_type {
                                                0 => {
                                                    match bincode::deserialize::<GameOperation> (&session.buf[session.cur_head..session.cur_head + header.len as usize]) {
                                                        Ok(game_op) => {
                                                            println!("recv provide:{:?} target:{}", game_op.provide_cards, game_op.target);
                                                        },
                                                        Err(err) => {
                                                            println!("parse message err: {:?}", err);
                                                        }
                                                    }
                                                },
                                                _ => (),
                                            }
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

    pub fn start(&mut self) {
        let mut rt =  Runtime::new().unwrap();
        rt.block_on(self._run());
    }
}
