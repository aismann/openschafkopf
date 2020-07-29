// adapted from https://github.com/sdroege/async-tungstenite/blob/master/examples/server.rs

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use crate::util::*;
use crate::game::*;
use crate::rules::*;
use crate::rules::ruleset::{SRuleSet, allowed_rules};

use futures::prelude::*;
use futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    future, pin_mut,
};
use serde::{Serialize, Deserialize};

use async_std::{
    net::{TcpListener, TcpStream},
    task,
};
use async_tungstenite::tungstenite::protocol::Message;
use crate::primitives::*;

#[derive(Debug, Serialize, Deserialize)]
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
type VGamePhaseActivePlayerInfo<'a> = VGamePhaseGeneric<
    (&'a SDealCards, <SDealCards as TGamePhase>::ActivePlayerInfo),
    (&'a SGamePreparations, <SGamePreparations as TGamePhase>::ActivePlayerInfo),
    (&'a SDetermineRules, <SDetermineRules as TGamePhase>::ActivePlayerInfo),
    (&'a SGame, <SGame as TGamePhase>::ActivePlayerInfo),
    (&'a SGameResult, <SGameResult as TGamePhase>::ActivePlayerInfo),
>;
type SActivelyPlayableRulesIdentifier = String;
#[derive(Serialize)]
enum VGameAction {
    Stoss,
    Zugeben(SCard),
}
type VGamePhaseAction = VGamePhaseGeneric<
    /*DealCards announce_doubling*/ /*b_doubling*/bool,
    /*GamePreparations announce_game*/Option<SActivelyPlayableRulesIdentifier>,
    /*DetermineRules*/Option<SActivelyPlayableRulesIdentifier>,
    /*Game*/VGameAction,
    /*GameResult*/(), // TODO? should players be able to "accept" result?
>;

impl VGamePhase {
    fn which_player_can_do_something(&self) -> Option<VGamePhaseActivePlayerInfo> {
        use VGamePhaseGeneric::*;
        fn internal<GamePhase: TGamePhase>(gamephase: &GamePhase) -> Option<(&GamePhase, GamePhase::ActivePlayerInfo)> {
            gamephase.which_player_can_do_something()
                .map(|activeplayerinfo| (gamephase, activeplayerinfo))
        }
        match self {
            DealCards(dealcards) => internal(dealcards).map(DealCards),
            GamePreparations(gamepreparations) => internal(gamepreparations).map(GamePreparations),
            DetermineRules(determinerules) => internal(determinerules).map(DetermineRules),
            Game(game) => internal(game).map(Game),
            GameResult(gameresult) => internal(gameresult).map(GameResult),
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
    Ask(Vec<VGamePhaseAction>),
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
            let str_msg = debug_verify!(msg.to_text()).unwrap();
            let mut peers = debug_verify!(peers.lock()).unwrap();
            let oepi = EPlayerIndex::values()
                .find(|epi| peers.mapepiopeer[*epi].as_ref().map(|peer| peer.sockaddr)==Some(sockaddr));
            println!(
                "Received a message from {} ({:?}): {}",
                sockaddr,
                oepi,
                str_msg,
            );
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
                if let Some(whichplayercandosomething) = verify!(gamephase.which_player_can_do_something()) {
                    use VGamePhaseGeneric::*;
                    match whichplayercandosomething {
                        DealCards((_dealcards, epi_doubling)) => {
                            peers.for_each(|oepi| {
                                if Some(epi_doubling)==oepi {
                                    VMessage::Ask(
                                        [true, false]
                                            .iter()
                                            .map(|b_doubling| 
                                                VGamePhaseAction::DealCards(*b_doubling)
                                            )
                                            .collect()
                                    )
                                } else {
                                    VMessage::Info(format!("Asking {:?} for doubling", epi_doubling))
                                }
                            });
                        },
                        GamePreparations((gamepreparations, epi_announce_game)) => {
                            peers.for_each(|oepi| {
                                if Some(epi_announce_game)==oepi {
                                    VMessage::Ask(
                                        allowed_rules(
                                            &gamepreparations.ruleset.avecrulegroup[epi_announce_game],
                                            gamepreparations.fullhand(epi_announce_game),
                                        )
                                            .map(|orules|
                                                VGamePhaseAction::GamePreparations(orules.map(TActivelyPlayableRules::to_string))
                                            )
                                            .collect()
                                    )
                                } else {
                                    VMessage::Info(format!("Asking {:?} for game", epi_announce_game))
                                }
                            });
                        },
                        DetermineRules((determinerules, (epi_determine, vecrulegroup))) => {
                            peers.for_each(|oepi| {
                                if Some(epi_determine)==oepi {
                                    VMessage::Ask(
                                        allowed_rules(
                                            &vecrulegroup,
                                            determinerules.fullhand(epi_determine),
                                        )
                                            .map(|orules|
                                                VGamePhaseAction::DetermineRules(orules.map(TActivelyPlayableRules::to_string))
                                            )
                                            .collect()
                                    )
                                } else {
                                    VMessage::Info(format!("Re-Asking {:?} for game", epi_determine))
                                }
                            });
                        },
                        Game((game, (epi_card, vecepi_stoss))) => {
                            peers.for_each(|oepi| {
                                let mut vecmessage = Vec::new();
                                if Some(epi_card)==oepi {
                                    for card in game.ahand[epi_card].cards().iter() {
                                        vecmessage.push(VGamePhaseAction::Game(VGameAction::Zugeben(*card)));
                                    }
                                }
                                if oepi.map_or(false, |epi| vecepi_stoss.contains(&epi)) {
                                    vecmessage.push(VGamePhaseAction::Game(VGameAction::Stoss));
                                }
                                if vecmessage.is_empty() {
                                    VMessage::Info(format!("Asking {:?} for card", epi_card))
                                } else {
                                    VMessage::Ask(vecmessage)
                                }
                            });
                        },
                        GameResult((_gameresult, ())) => {
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

