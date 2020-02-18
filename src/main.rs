
pub mod game;
pub mod server_net;
use std::thread;

fn main() {
    println!("Hello, world!");
    let mut p1 = game::player::Player {
        id: 1,
        score: 0,
        fd: 0,
    };
    let mut p2 = game::player::Player {
        id: 2,
        score: 0,
        fd: 0,
    };
    let mut p3 = game::player::Player {
        id: 3,
        score: 0,
        fd: 0,
    };
    let mut p4 = game::player::Player {
        id: 4,
        score: 0,
        fd: 0,
    };
    let mut round = game::Game::new(vec![p1, p2, p3, p4]);
    //round.init();
    //round.start();
    let mut server = server_net::MultiThreadServer::new("0.0.0.0:8890".to_string());
    let t1 = thread::spawn(move || {
        server.start();
    });
    t1.join();
}
