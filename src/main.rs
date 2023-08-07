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

    let node_count = 10;
    // construct a token ring
    let mut nodes = vec![Node::new()];
    for i in 1..node_count {
        let node = Node::new();
        nodes[i - 1].connect(&node).unwrap();
        nodes.push(node);
    }
    nodes.last().unwrap().connect(&nodes[0]).unwrap();


    let node = &nodes[0];
    node.init().unwrap();
    thread::sleep(Duration::from_millis(1000));

    let (tx, rx) = node.channel();
    //for i in 0..node_count {
    //    node.ping(i).unwrap();
    //}

    for i in 1..10 {
        node.store(format!("key[{i}]"), format!("doc[{i}]")).unwrap();
    }

    thread::sleep(Duration::from_millis(1000));

    for i in 1..10 {
        node.load(format!("key[{i}]")).unwrap();
    }


    thread::sleep(Duration::from_millis(1000));
    node.shutdown().unwrap();

    thread::sleep(Duration::from_millis(1000));

    while let Ok(token) = rx.recv() {
        println!("Received {token:?}");
    }

    for node in nodes {
        node.join().unwrap();
    }
}
