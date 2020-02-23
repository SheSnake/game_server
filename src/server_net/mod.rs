extern crate tokio;
extern crate serde_derive;
extern crate serde;
extern crate serde_bytes;
extern crate bincode;
use std::mem;
use std::sync::{Arc};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{ Sender };
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{ WriteHalf, ReadHalf };
use tokio::prelude::*;
use std::collections::HashMap;

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

pub async fn server_run(bind_addr: String, sender: Sender<Vec<u8>>, writefd_map: Arc<Mutex<HashMap<i64, WriteHalf<TcpStream>>>>) {
    let mut listener = TcpListener::bind(bind_addr).await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let mut sender = sender.clone();
        let writefd_map = writefd_map.clone();
        let (readfd,  writefd) = tokio::io::split(socket);
        //self.st.push(writefd);
        tokio::spawn(async move {
            let mut authorized = false;
            let mut user_id = -1;
            while !authorized {
                match session.fd.read(&mut session.buf[session.cur_tail..]).await {
                    Ok(n) if n == 0 => {
                        println!("session close");
                        return;
                    }
                    Ok(n) => {
                        session.cur_tail += n as usize;
                        session.cur_size += n as usize;
                        if session.reach_buf_end() {
                            session.rollbuf();
                        }
                        let header_size = mem::size_of::<Header>();
                        // for each read, handle all the finished recv meesage
                        while session.cur_size >= header_size {
                            unsafe {
                                match bincode::deserialize::<Header> (&session.buf[session.cur_head..session.cur_head + header_size]) {
                                    Ok(header) => {
                                        println!("has read header, cur_head:{}, cur size:{}, need_len:{}", session.cur_head, session.cur_size, &header.len);
                                        if session.cur_size >= header.len as usize {
                                            let a: &[u8] = &session.buf[session.cur_head..session.cur_head + header.len as usize];
                                            let msg: Vec<u8> = a.iter().cloned().collect();
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

            let mut session = ClientSession::new(readfd);
            {
                let mut map = writefd_map.lock().await;
                map.insert(123, writefd);
            }
            loop {
                match session.fd.read(&mut session.buf[session.cur_tail..]).await {
                    Ok(n) if n == 0 => {
                        println!("session close");
                        return;
                    }
                    Ok(n) => {
                        session.cur_tail += n as usize;
                        session.cur_size += n as usize;
                        if session.reach_buf_end() {
                            session.rollbuf();
                        }
                        let header_size = mem::size_of::<Header>();
                        // for each read, handle all the finished recv meesage
                        while session.cur_size >= header_size {
                            unsafe {
                                match bincode::deserialize::<Header> (&session.buf[session.cur_head..session.cur_head + header_size]) {
                                    Ok(header) => {
                                        println!("has read header, cur_head:{}, cur size:{}, need_len:{}", session.cur_head, session.cur_size, &header.len);
                                        if session.cur_size >= header.len as usize {
                                            let a: &[u8] = &session.buf[session.cur_head..session.cur_head + header.len as usize];
                                            let msg: Vec<u8> = a.iter().cloned().collect();
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
