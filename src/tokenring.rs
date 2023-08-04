use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use threadpool::ThreadPool;

#[derive(Debug)]
pub enum Token {
    NUMBER(u64),
    TEXT(String),
    SHUTDOWN
}

type Node = fn(Receiver<Token>, Sender<Token>);

pub struct TokenRing {
    pool: ThreadPool,
    pub rx: Receiver<Token>,
    pub tx: Sender<Token>,
}

impl TokenRing {
    pub fn new(count: usize, node: Node) -> TokenRing {
        let pool = ThreadPool::new(count);

        let mut rxs = vec![];
        let mut txs = vec![];

        for i in 0..count + 1 {
            let (tx, rx) = mpsc::channel();
            rxs.push(rx);
            txs.push(tx);
        }

        let main_tx = txs.remove(0);
        let main_rx = rxs.remove(rxs.len() - 1);


        for i in 0..count {
            let tx = txs.remove(0);
            let rx = rxs.remove(0);
            pool.execute(move || node(rx, tx));
        }


        TokenRing {
            pool,
            rx: main_rx,
            tx: main_tx,
        }
    }

    pub fn join(&self) {
        self.pool.join()
    }
}



