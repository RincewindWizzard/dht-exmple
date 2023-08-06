use std::sync::mpsc::channel;
use std::thread;
use std::thread::JoinHandle;
use crossbeam_channel::{Receiver, RecvError, Sender, SendError};
use log::debug;
use crate::tokenring::Token::{ConnectRx, ConnectTx};

type NodeId = usize;


#[derive(Debug, Clone)]
pub enum Token {
    TEXT(String),
    /// Initializing nodes with ids
    /// The intialized node saves its id and forwards INIT(id + 1) to the next node
    INIT(NodeId),
    PING(NodeId, NodeId),
    PONG(NodeId, NodeId),
    ConnectTx(Sender<Token>),
    ConnectRx(Receiver<Token>),
    SHUTDOWN,
}

fn is_internal(token: &Token) -> bool {
    match token {
        Token::TEXT(_) => { false }
        Token::INIT(_) => { true }
        Token::PING(_, _) => { false }
        Token::PONG(_, _) => { false }
        Token::SHUTDOWN => { true }
        Token::ConnectTx(_) => { true }
        Token::ConnectRx(_) => { true }
    }
}

#[derive(PartialEq, Clone)]
pub enum RouteType {
    EXTERNAL,
    INTERNAL,
    BROADCAST,
}

pub trait NodeLogik {
    fn run(node: Node);
}


pub struct Node {
    id: Option<NodeId>,
    tx: Vec<(RouteType, Sender<Token>)>,
    rx: Vec<(RouteType, Receiver<Token>)>,
}

pub struct NodeHandle {
    tx: Sender<Token>,
    rx: Receiver<Token>,
    thread: JoinHandle<()>,
}

impl NodeHandle {
    pub fn join(self) -> std::thread::Result<()> {
        self.thread.join()
    }
    pub fn channel(&self) -> (Sender<Token>, Receiver<Token>) {
        (self.tx.clone(), self.rx.clone())
    }
    pub fn connect(&self, other: &NodeHandle) -> anyhow::Result<()> {
        let (channel_tx, channel_rx) = crossbeam_channel::unbounded();
        self.tx.send(ConnectTx(channel_tx))?;
        other.tx.send(ConnectRx(channel_rx))?;
        Ok(())
    }
    pub fn ping(&self, dst: NodeId) -> Result<(), SendError<Token>> {
        self.tx.send(Token::PING(0, dst))
    }
    pub fn init(&self) -> Result<(), SendError<Token>> {
        self.tx.send(Token::INIT(1))
    }
    pub fn shutdown(&self) -> Result<(), SendError<Token>> {
        self.tx.send(Token::SHUTDOWN)
    }
}


impl Node {
    pub fn new() -> NodeHandle {
        let (external_out_tx, external_out_rx) = crossbeam_channel::unbounded();
        let (external_in_tx, external_in_rx) = crossbeam_channel::unbounded();
        let mut node = Node {
            id: None,
            tx: vec![(RouteType::EXTERNAL, external_out_tx)],
            rx: vec![(RouteType::EXTERNAL, external_in_rx)],
        };

        let handle = thread::spawn(move || {
            node.run()
        });


        NodeHandle {
            tx: external_in_tx,
            rx: external_out_rx,
            thread: handle,
        }
    }

    fn node_name(&self) -> String {
        if let Some(id) = self.id {
            format!("node {id}")
        } else {
            format!("unknown node")
        }
    }

    fn run(&mut self) {
        while let Ok((route_type, token)) = self.recv() {
            match token {
                Token::INIT(new_id) => {
                    if let None = self.id {
                        self.id = Some(new_id);
                        debug!("initialized {}", self.node_name());
                        self.forward(Token::INIT(new_id + 1), RouteType::INTERNAL).unwrap();
                    } else {
                        self.forward(token, RouteType::INTERNAL).unwrap();
                    }
                }
                Token::ConnectRx(rx) => {
                    self.rx.push((RouteType::INTERNAL, rx));
                }
                Token::ConnectTx(tx) => {
                    if let Some(id) = self.id {
                        tx.send(Token::INIT(id + 1)).unwrap();
                    }

                    self.tx.push((RouteType::INTERNAL, tx));
                }
                Token::SHUTDOWN => {
                    debug!("Shutdown {}", self.node_name());
                    self.forward(token, RouteType::INTERNAL).unwrap();
                    if route_type == RouteType::INTERNAL {
                        break;
                    }
                }
                Token::PING(src, dst) => {
                    debug!("{} forwarded ping to dst {dst}", self.node_name());
                    if let Some(id) = self.id {
                        let src = if src == 0 {
                            id
                        } else { src };

                        if id == dst {
                            let (src, dst) = (dst, src);
                            self.forward(
                                Token::PONG(src, dst),
                                if dst == id { RouteType::EXTERNAL } else { RouteType::INTERNAL },
                            ).unwrap();
                        } else {
                            self.forward(
                                Token::PING(src, dst),
                                if dst == id { RouteType::EXTERNAL } else { RouteType::INTERNAL },
                            ).unwrap();
                        }
                    } else {
                        self.forward(token, RouteType::INTERNAL).unwrap();
                    }
                }
                Token::PONG(src, dst) => {
                    debug!("{} forwarded pong to dst {dst}", self.node_name());
                    if let Some(id) = self.id {
                        if dst == id {
                            self.forward(token, RouteType::EXTERNAL).unwrap();
                        } else {
                            self.forward(token, RouteType::INTERNAL).unwrap();
                        }
                    }
                }
                _ => {
                    debug!("{} forwards {token:?}", self.node_name());
                    let dst = if route_type == RouteType::INTERNAL || route_type == RouteType::BROADCAST {
                        RouteType::BROADCAST
                    } else {
                        RouteType::INTERNAL
                    };
                    self.forward(token, dst).unwrap();
                }
            }
        };
    }

    pub fn forward(&mut self, token: Token, dst: RouteType) -> Result<(), anyhow::Error> {
        let mut failed = vec![];
        for (i, (route_type, tx)) in self.tx.iter().enumerate() {
            if dst == RouteType::BROADCAST || dst == *route_type {
                let result = tx.send(token.clone());
                if let Err(SendError(token)) = result {
                    debug!("{} Remove tx index {}", self.node_name(), i);
                    failed.push(i);
                }
            }
        }

        failed.reverse();
        for i in failed {
            self.tx.remove(i);
        }
        Ok(())
    }

    pub fn send(&mut self, token: Token) -> Result<(), anyhow::Error> {
        let dst = if is_internal(&token) {
            RouteType::INTERNAL
        } else {
            RouteType::BROADCAST
        };
        self.forward(token, dst)
    }

    pub fn recv(&self) -> Result<(RouteType, Token), RecvError> {
        let mut sel = crossbeam_channel::Select::new();

        for (route_type, rx) in &self.rx {
            sel.recv(&rx);
        }
        // Complete the selected operation.
        let oper = sel.select();
        let index = oper.index();
        let (route_type, rx) = &self.rx[index];
        let route_type = route_type.clone();

        let token = oper.recv(rx)?;
        Ok((route_type, token))
    }
}

