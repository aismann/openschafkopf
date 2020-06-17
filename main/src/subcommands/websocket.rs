// adapted from https://github.com/sdroege/async-tungstenite/blob/master/examples/server.rs

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use crate::util::*;

use futures::prelude::*;
use futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    future, pin_mut,
};

use async_std::{
    net::{TcpListener, TcpStream},
    task,
};
use async_tungstenite::tungstenite::protocol::Message;
use crate::primitives::*;

#[derive(Debug)]
struct SPeer {
    sockaddr: SocketAddr,
    txmsg: UnboundedSender<Message>,
}

#[derive(Default, Debug)]
struct SPeers {
    mapepiopeer: EnumMap<EPlayerIndex, Option<SPeer>>, // active
    vecpeer: Vec<SPeer>, // inactive
}
impl SPeers {
    fn insert(&mut self, peer: SPeer) {
        match self.mapepiopeer
            .iter_mut()
            .find(|opeer| opeer.is_none())
        {
            Some(opeer) => {
                assert!(opeer.is_none());
                *opeer = Some(peer)
            },
            None => {
                self.vecpeer.push(peer);
            }
        }
    }

    fn remove(&mut self, sockaddr: &SocketAddr) {
        for epi in EPlayerIndex::values() {
            if self.mapepiopeer[epi].as_ref().map(|peer| peer.sockaddr)==Some(*sockaddr) {
                self.mapepiopeer[epi] = None;
            }
        }
        self.vecpeer.retain(|peer| peer.sockaddr!=*sockaddr);
    }

    fn for_each(&self, mut f: impl FnMut(Option<EPlayerIndex>, UnboundedSender<Message>)) {
        for epi in EPlayerIndex::values() {
            if let Some(peer) = self.mapepiopeer[epi].as_ref() {
                f(Some(epi), peer.txmsg.clone());
            }
        }
        for peer in &self.vecpeer {
            f(None, peer.txmsg.clone());
        }
    }
}

async fn handle_connection(peers: Arc<Mutex<SPeers>>, tcpstream: TcpStream, sockaddr: SocketAddr) {
    println!("Incoming TCP connection from: {}", sockaddr);
    let wsstream = debug_verify!(async_tungstenite::accept_async(tcpstream).await).unwrap();
    println!("WebSocket connection established: {}", sockaddr);
    // Insert the write part of this peer to the peer map.
    let (txmsg, rxmsg) = unbounded();
    debug_verify!(peers.lock()).unwrap().insert(SPeer{sockaddr, txmsg});
    let (sink_ws_out, stream_ws_in) = wsstream.split();
    let broadcast_incoming = stream_ws_in
        .try_filter(|msg| {
            // Broadcasting a Close message from one client
            // will close the other clients.
            future::ready(!msg.is_close())
        })
        .try_for_each(|msg| {
            println!(
                "Received a message from {}: {}",
                sockaddr,
                debug_verify!(msg.to_text()).unwrap()
            );
            debug_verify!(peers.lock())
                .unwrap()
                .for_each(|oepi, tx_peer| {
                    debug_verify!(tx_peer.unbounded_send(format!("{:?}: {}", oepi, msg).into())).unwrap();
                });
            future::ok(())
        });
    let receive_from_others = rxmsg.map(Ok).forward(sink_ws_out);
    pin_mut!(broadcast_incoming, receive_from_others); // TODO Is this really needed?
    future::select(broadcast_incoming, receive_from_others).await;
    println!("{} disconnected", &sockaddr);
    debug_verify!(peers.lock()).unwrap().remove(&sockaddr);
}

async fn internal_run() -> Result<(), Error> {
    let str_addr = "127.0.0.1:8080";
    let peers = Arc::new(Mutex::new(SPeers::default()));
    // Create the event loop and TCP listener we'll accept connections on.
    let listener = debug_verify!(TcpListener::bind(&str_addr).await).unwrap();
    println!("Listening on: {}", str_addr);
    // Let's spawn the handling of each connection in a separate task.
    while let Ok((tcpstream, sockaddr)) = listener.accept().await {
        task::spawn(handle_connection(peers.clone(), tcpstream, sockaddr));
    }
    Ok(())
}

pub fn run() -> Result<(), Error> {
    task::block_on(internal_run())
}

