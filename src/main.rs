mod tokenring;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use threadpool::ThreadPool;
use crate::tokenring::{Token, TokenRing};

enum Message {
    NUMBER(u64),
    TEXT(String),
}

fn producer(tx: Sender<Message>) {
    for i in 1..10 {
        tx.send(Message::NUMBER(i)).unwrap();
    }
}

fn mapper(rx: Receiver<Message>, tx: Sender<Message>) {
    let mut sum = 0;
    while let Ok(msg) = rx.recv() {
        let result = match msg {
            Message::NUMBER(number) => {
                sum += number;
            }
            _ => { tx.send(msg).unwrap(); }
        };
    }
    tx.send(Message::NUMBER(sum)).unwrap();
}


fn consumer(rx: Receiver<Message>) {
    while let Ok(msg) = rx.recv() {
        let log = match msg {
            Message::NUMBER(number) => { format!("{number}") }
            Message::TEXT(text) => { text }
        };
        println!("Received: {log}");
    }
}

fn node(rx: Receiver<Token>, tx: Sender<Token>) {
    while let Ok(token) = rx.recv() {
        match token {
            Token::SHUTDOWN => {
                break;
            }
            _ => {
                tx.send(token).unwrap();
            }
        }
    };
}

fn main() {
    println!("Hello, world!");

    let ring = TokenRing::new(10, node);
    ring.tx.send(Token::TEXT(format!("Hello Token Ring!"))).unwrap();
    ring.tx.send(Token::SHUTDOWN).unwrap();


    while let Ok(token) = ring.rx.recv() {
        if let Token::SHUTDOWN = token {
            println!("Shutdown token ring.");
            break;
        }
        println!("Received: {token:?}");
    }

    ring.join();
}
