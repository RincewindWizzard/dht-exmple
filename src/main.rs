mod tokenring;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use log::debug;
use stderrlog::LogLevelNum;
use threadpool::ThreadPool;
use crate::tokenring::{Token, TokenRing};

fn node(rx: Receiver<Token>, tx: Sender<Token>) {
    let mut id = 0;
    while let Ok(token) = rx.recv() {
        match token {
            Token::INIT(new_id) => {
                id = new_id;
                debug!("node {id:02}: initialized node");
                tx.send(Token::INIT(id + 1)).unwrap();
            }
            _ => {
                tx.send(token).unwrap();
            }
        }
    };
}

fn main() {
    stderrlog::new()
        .module(module_path!())
        .quiet(false)
        .verbosity(LogLevelNum::Debug) // show warnings and above
        .timestamp(stderrlog::Timestamp::Millisecond)
        .init()
        .unwrap();

    let mut ring = TokenRing::new(10, node);
    ring.send(Token::TEXT(format!("Hello Token Ring!")));
    ring.shutdown();


    while let Ok(token) = ring.rx.recv() {
        println!("Received: {token:?}");
    }

    ring.join();
}
