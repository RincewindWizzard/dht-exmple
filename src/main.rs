use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use threadpool::ThreadPool;

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


fn main() {
    println!("Hello, world!");


    let pool = ThreadPool::new(4);
    let (producer_out, mapper_in) = mpsc::channel();
    let (mapper_out, consumer_in) = mpsc::channel();

    pool.execute(move || producer(producer_out));
    pool.execute(move || mapper(mapper_in, mapper_out));
    pool.execute(move || consumer(consumer_in));

    pool.join();
}
