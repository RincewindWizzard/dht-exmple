use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use threadpool::ThreadPool;

#[derive(Debug)]
pub enum Token {
    NUMBER(u64),
    TEXT(String),
    /// Initializing nodes with ids
    /// The intialized node saves its id and forwards INIT(id + 1) to the next node
    INIT(NodeId),
}

type Node = fn(Receiver<Token>, Sender<Token>);
type NodeId = usize;

pub struct TokenRing {
    pool: ThreadPool,
    pub rx: Receiver<Token>,
    pub tx: Option<Sender<Token>>,
}

impl TokenRing {
    pub fn new(count: usize, node: Node) -> TokenRing {
        let pool = ThreadPool::new(count + 1);

        let mut rxs = vec![];
        let mut txs = vec![];

        for i in 0..count + 1 {
            let (tx, rx) = mpsc::channel();
            rxs.push(rx);
            txs.push(tx);
        }

        let ring_tx = txs.remove(0);
        let ring_rx = rxs.remove(rxs.len() - 1);


        for i in 0..count {
            let tx = txs.remove(0);
            let rx = rxs.remove(0);
            pool.execute(move || node(rx, tx));
        }

        let (filter_out_tx, main_rx) = mpsc::channel();


        pool.execute(move || {
            while let Ok(token) = ring_rx.recv() {
                match token {
                    Token::INIT(_) => {}
                    _ => {
                        filter_out_tx.send(token).unwrap();
                    }
                }
            }
        });


        let main_tx = ring_tx;

        let tokenring = TokenRing {
            pool,
            rx: main_rx,
            tx: Some(main_tx),
        };
        tokenring.send(Token::INIT(1));
        tokenring
    }

    pub fn shutdown(&mut self) {
        self.tx = None;
    }

    pub fn send(&self, token: Token) {
        let tx = self.tx.as_ref().unwrap();
        tx.send(token).unwrap();
    }

    pub fn join(&self) {
        self.pool.join()
    }
}



