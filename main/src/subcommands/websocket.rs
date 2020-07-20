// adapted from https://github.com/sdroege/async-tungstenite/blob/master/examples/server.rs

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use crate::util::*;
use crate::game::*;
use crate::rules::*;
use crate::rules::ruleset::SRuleSet;

use futures::prelude::*;
use futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    future, pin_mut,
};
use serde::Serialize;

use async_std::{
    net::{TcpListener, TcpStream},
    task,
};
use async_tungstenite::tungstenite::protocol::Message;
use crate::primitives::*;

#[derive(Debug)]
enum VGamePhaseGeneric<DealCards, GamePreparations, DetermineRules, Game, GameResult> {
    DealCards(DealCards),
    GamePreparations(GamePreparations),
    DetermineRules(DetermineRules),
    Game(Game),
    GameResult(GameResult),
}

type VGamePhase = VGamePhaseGeneric<
    SDealCards,
    SGamePreparations,
    SDetermineRules,
    SGame,
    SGameResult,
>;
type VGamePhaseActivePlayerInfo = VGamePhaseGeneric<
    <SDealCards as TGamePhase>::ActivePlayerInfo,
    <SGamePreparations as TGamePhase>::ActivePlayerInfo,
    <SDetermineRules as TGamePhase>::ActivePlayerInfo,
    <SGame as TGamePhase>::ActivePlayerInfo,
    <SGameResult as TGamePhase>::ActivePlayerInfo,
>;
enum VDetermineRulesAction {
    AnnounceGame(EPlayerIndex, Box<dyn TActivelyPlayableRules>),
    Resign(EPlayerIndex),
}
enum VGameAction {
    Stoss(EPlayerIndex),
    Zugeben(SCard, EPlayerIndex),
}
type VGamePhaseAction = VGamePhaseGeneric<
    /*DealCards announce_doubling*/(EPlayerIndex, /*b_doubling*/bool),
    /*GamePreparations announce_game*/(EPlayerIndex, Option<Box<dyn TActivelyPlayableRules>>),
    /*DetermineRules*/VDetermineRulesAction,
    /*Game*/VGameAction,
    /*GameResult*/(), // TODO? should players be able to "accept" result?
>;

impl VGamePhase {
    fn which_player_can_do_something(&self) -> Option<VGamePhaseActivePlayerInfo> {
        use VGamePhaseGeneric::*;
        match self {
            DealCards(dealcards) => dealcards.which_player_can_do_something().map(DealCards),
            GamePreparations(gamepreparations) => gamepreparations.which_player_can_do_something().map(GamePreparations),
            DetermineRules(determinerules) => determinerules.which_player_can_do_something().map(DetermineRules),
            Game(game) => game.which_player_can_do_something().map(Game),
            GameResult(gameresult) => gameresult.which_player_can_do_something().map(GameResult),
        }
    }
}

#[derive(Debug)]
struct SPeer {
    sockaddr: SocketAddr,
    txmsg: UnboundedSender<Message>,
    n_money: isize,
}

fn static_ruleset() -> SRuleSet {
    debug_verify!(SRuleSet::from_string(
        r"
        base-price=10
        solo-price=50
        lauf-min=3
        [rufspiel]
        [solo]
        [wenz]
        lauf-min=2
        [stoss]
        max=3
        ",
    )).unwrap()
}

#[derive(Default, Debug)]
struct SPeers {
    mapepiopeer: EnumMap<EPlayerIndex, Option<SPeer>>, // active
    vecpeer: Vec<SPeer>, // inactive
    ogamephase: Option<VGamePhase>,
    n_stock: isize, // TODO would that be better within VGamePhase?
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
        if self.ogamephase.is_none()
            && self.mapepiopeer
                .iter()
                .all(|opeer| opeer.is_some())
        {
            self.ogamephase = Some(VGamePhase::DealCards(SDealCards::new(
                static_ruleset(),
                self.n_stock,
            )));
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

    fn for_each(&self, mut f: impl FnMut(Option<EPlayerIndex>)->VMessage) {
        let mut communicate = |oepi, txmsg: UnboundedSender<_>| {
            let msg = f(oepi);
            debug_verify!(txmsg.unbounded_send(
                debug_verify!(serde_json::to_string(&(oepi, msg))).unwrap().into()
            )).unwrap();
        };
        for epi in EPlayerIndex::values() {
            if let Some(peer) = self.mapepiopeer[epi].as_ref() {
                communicate(Some(epi), peer.txmsg.clone());
            }
        }
        for peer in &self.vecpeer {
            communicate(None, peer.txmsg.clone());
        }
    }
}

#[derive(Serialize)]
enum VMessage {
    Info(String),
}

async fn handle_connection(peers: Arc<Mutex<SPeers>>, tcpstream: TcpStream, sockaddr: SocketAddr) {
    println!("Incoming TCP connection from: {}", sockaddr);
    let wsstream = debug_verify!(async_tungstenite::accept_async(tcpstream).await).unwrap();
    println!("WebSocket connection established: {}", sockaddr);
    // Insert the write part of this peer to the peer map.
    let (txmsg, rxmsg) = unbounded();
    debug_verify!(peers.lock()).unwrap().insert(SPeer{sockaddr, txmsg, n_money: 0});
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
            let mut peers = debug_verify!(peers.lock()).unwrap();
            if let Some(mut gamephase) = peers.ogamephase.take() /*TODO take necessary here?*/ {
                while gamephase.which_player_can_do_something().is_none() {
                    use VGamePhaseGeneric::*;
                    gamephase = match gamephase {
                        DealCards(dealcards) => match dealcards.finish() {
                            Ok(gamepreparations) => GamePreparations(gamepreparations),
                            Err(dealcards) => DealCards(dealcards),
                        },
                        GamePreparations(gamepreparations) => match gamepreparations.finish() {
                            Ok(VGamePreparationsFinish::DetermineRules(determinerules)) => DetermineRules(determinerules),
                            Ok(VGamePreparationsFinish::DirectGame(game)) => Game(game),
                            Ok(VGamePreparationsFinish::Stock(n_stock)) => {
                                for epi in EPlayerIndex::values() {
                                    if let Some(ref mut peer) = peers.mapepiopeer[epi] {
                                        peer.n_money -= n_stock;
                                    }
                                }
                                peers.n_stock += n_stock * EPlayerIndex::SIZE.as_num::<isize>();
                                DealCards(SDealCards::new(static_ruleset(), peers.n_stock))
                            },
                            Err(gamepreparations) => GamePreparations(gamepreparations),
                        }
                        DetermineRules(determinerules) => match determinerules.finish() {
                            Ok(game) => Game(game),
                            Err(determinerules) => DetermineRules(determinerules),
                        },
                        Game(game) => match game.finish() {
                            Ok(gameresult) => GameResult(gameresult),
                            Err(game) => Game(game),
                        },
                        GameResult(gameresult) => match gameresult.finish() {
                            Ok(gameresult) | Err(gameresult) => GameResult(gameresult),
                        },
                    };
                    peers.for_each(|oepi| {
                        VMessage::Info(format!("{:?}: Transitioning to next phase", oepi).into())
                    });
                }
                if let Some(activeplayerinfo) = verify!(gamephase.which_player_can_do_something()) {
                    use VGamePhaseGeneric::*;
                    match activeplayerinfo {
                        DealCards(epi_doubling) => {
                            peers.for_each(|oepi| {
                                if Some(epi_doubling)==oepi {
                                    VMessage::Info(format!("Double?"))
                                } else {
                                    VMessage::Info(format!("Asking {:?} for doubling", epi_doubling))
                                }
                            });
                        },
                        GamePreparations(epi_announce_game) => {
                            peers.for_each(|oepi| {
                                if Some(epi_announce_game)==oepi {
                                    VMessage::Info(format!("Announce game?"))
                                } else {
                                    VMessage::Info(format!("Asking {:?} for game", epi_announce_game))
                                }
                            });
                        },
                        DetermineRules((epi_determine, _vecrulegroup)) => {
                            peers.for_each(|oepi| {
                                if Some(epi_determine)==oepi {
                                    VMessage::Info(format!("Re-Announce game?"))
                                } else {
                                    VMessage::Info(format!("Re-Asking {:?} for game", epi_determine))
                                }
                            });
                        },
                        Game((epi_card, vecepi_stoss)) => {
                            peers.for_each(|oepi| {
                                match (Some(epi_card)==oepi, oepi.map_or(false, |epi| vecepi_stoss.contains(&epi))) {
                                    (true, true) => {
                                        VMessage::Info(format!("Card?, Stoss?"))
                                    },
                                    (true, false) => {
                                        VMessage::Info(format!("Card?"))
                                    },
                                    (false, true) => {
                                        VMessage::Info(format!("Stoss?"))
                                    },
                                    (false, false) => {
                                        VMessage::Info(format!("Asking {:?} for card", epi_card))
                                    },
                                }
                            });
                        },
                        GameResult(()) => {
                            peers.for_each(|_oepi| {
                                VMessage::Info(format!("Game finished"))
                            });
                        },
                    }
                }
                peers.ogamephase = Some(gamephase);
                assert!(peers.ogamephase.is_some());
            } else {
                peers.for_each(|_oepi| VMessage::Info("Waiting for more players.".into()));
            }
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

