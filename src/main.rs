mod tokenring;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;
use log::debug;
use stderrlog::LogLevelNum;
use threadpool::ThreadPool;
use crate::tokenring::{Node, RouteType, Token};
use crate::tokenring::Token::TEXT;

fn main() {
    stderrlog::new()
        .module(module_path!())
        .quiet(false)
        .verbosity(LogLevelNum::Debug) // show warnings and above
        .timestamp(stderrlog::Timestamp::Millisecond)
        .init()
        .unwrap();

    // construct a token ring
    let mut nodes = vec![Node::new()];
    for i in 1..10 {
        let node = Node::new();
        nodes[i - 1].connect(&node).unwrap();
        nodes.push(node);
    }
    nodes.last().unwrap().connect(&nodes[0]).unwrap();


    let node = &nodes[0];
    node.init().unwrap();
    thread::sleep(Duration::from_millis(1000));

    let (tx, rx) = node.channel();
    node.ping(5).unwrap();
    thread::sleep(Duration::from_millis(1000));
    let token = rx.recv().unwrap();
    println!("Received {token:?}");
    node.shutdown().unwrap();

    for node in nodes {
        node.join().unwrap();
    }
}
