extern crate tokio;
extern crate serde_derive;
extern crate serde;
extern crate serde_bytes;
extern crate bincode;
use std::mem;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::io::Read;
use std::io::{BufWriter, BufReader};
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
                    let n = match session.fd.read(&mut session.buf[session.cur_tail..]).await {
                        Ok(n) if n == 0 => {
                            println!("session close");
                            return;
                        }
                        Ok(n) => {
                            session.cur_tail += n;
                            session.cur_size += n;
                            println!("has read {}, need {}", session.cur_size, mem::size_of::<Header>());
                            if session.cur_size >= mem::size_of::<Header>() {
                                match bincode::deserialize::<Header> (&session.buf[session.cur_head..]) {
                                    Ok(header) => {
                                        println!("read header {}, {}", header.msg_type, header.len);
                                    },
                                    Err(err) => {
                                        println!("parse err: {:?}", err);
                                    },
                                }
                                //serde_bytes::deserialize::<std::vec::Vec<u8>,Header>(bytes);
                            }
                            println!("read {}: {}", n, String::from_utf8_lossy(&session.buf[..]));
                            n
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
