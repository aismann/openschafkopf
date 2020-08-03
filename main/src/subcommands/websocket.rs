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
use std::task::{Context, Poll, Waker};

use async_std::{
    net::{TcpListener, TcpStream},
    task,
};
use async_tungstenite::tungstenite::protocol::Message;
use crate::primitives::*;
use rand::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
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
struct STimeoutCmd {
    gamephaseaction: VGamePhaseAction,
    aborthandle: future::AbortHandle,
}

#[derive(Debug)]
struct SPeer {
    sockaddr: SocketAddr,
    txmsg: UnboundedSender<Message>,
    n_money: isize,
    otimeoutcmd: Option<STimeoutCmd>,
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
    fn insert(&mut self, self_mutex: Arc<Mutex<Self>>, peer: SPeer) {
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
            self.send_msg(
                self_mutex,
                /*oepi*/None,
                /*ogamephaseaction*/None,
            ); // To trigger game logic. TODO beautify instead of dummy msg.
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

    fn for_each(&mut self, mut f: impl FnMut(Option<EPlayerIndex>, &mut SPeer)->(Vec<SCard>, VMessage)) {
        let mut communicate = |oepi, peer: &mut SPeer| {
            let (veccard, msg) = f(oepi, peer);
            debug_verify!(peer.txmsg.unbounded_send(
                debug_verify!(serde_json::to_string(&(
                    oepi,
                    veccard.into_iter()
                        .map(|card| (card.to_string(), VGamePhaseAction::Game(VGameAction::Zugeben(card))))
                        .collect::<Vec<_>>(),
                    msg
                ))).unwrap().into()
            )).unwrap();
        };
        for epi in EPlayerIndex::values() {
            if let Some(ref mut peer) = self.mapepiopeer[epi].as_mut() {
                communicate(Some(epi), peer);
            }
        }
        for peer in self.vecpeer.iter_mut() {
            communicate(None, peer);
        }
    }

    fn send_msg(&mut self, /*TODO avoid this parameter*/self_mutex: Arc<Mutex<Self>>, oepi: Option<EPlayerIndex>, ogamephaseaction: Option<VGamePhaseAction>) {
        println!("send_msg({:?}, {:?})", oepi, ogamephaseaction);
        if let Some(mut gamephase) = self.ogamephase.take() /*TODO take necessary here?*/ {
            if let Some(epi) = oepi {
                fn handle_err<T, E: std::fmt::Display>(res: Result<T, E>) {
                    match res {
                        Ok(_) => {},
                        Err(e) => println!("Error {}", e),
                    };
                }
                if let Some(gamephaseaction) = ogamephaseaction {
                    if let Some(ref mut peer) = self.mapepiopeer[epi] {
                        use std::mem::discriminant;
                        match peer.otimeoutcmd.as_ref() {
                            None => (),
                            Some(timeoutcmd) => {
                                if discriminant(&gamephaseaction)==discriminant(&timeoutcmd.gamephaseaction) {
                                    timeoutcmd.aborthandle.abort();
                                    peer.otimeoutcmd = None;
                                }
                            },
                        }
                    }
                    match (&mut gamephase, gamephaseaction) {
                        (VGamePhase::DealCards(ref mut dealcards), VGamePhaseAction::DealCards(b_doubling)) => {
                            handle_err(dealcards.announce_doubling(epi, b_doubling));
                        },
                        (VGamePhase::GamePreparations(ref mut gamepreparations), VGamePhaseAction::GamePreparations(ref orulesid)) => {
                            if let Some(orules) = {
                                let oorules = allowed_rules(
                                    &gamepreparations.ruleset.avecrulegroup[epi],
                                    gamepreparations.fullhand(epi),
                                )
                                    .find(|orules|
                                        &orules.map(TActivelyPlayableRules::to_string)==orulesid
                                    )
                                    .map(|orules| orules.map(TActivelyPlayableRulesBoxClone::box_clone));
                                oorules.clone() // TODO needed?
                            } {
                                handle_err(gamepreparations.announce_game(epi, orules));
                            }
                        },
                        (VGamePhase::DetermineRules(ref mut determinerules), VGamePhaseAction::DetermineRules(ref orulesid)) => {
                            if let Some((_epi_active, vecrulegroup)) = determinerules.which_player_can_do_something() {
                                if let Some(orules) = {
                                    let oorules = allowed_rules(
                                        &vecrulegroup,
                                        determinerules.fullhand(epi),
                                    )
                                        .find(|orules|
                                            &orules.map(TActivelyPlayableRules::to_string)==orulesid
                                        );
                                    oorules.clone() // TODO clone needed?
                                } {
                                    handle_err(if let Some(rules) = orules {
                                        determinerules.announce_game(epi, TActivelyPlayableRulesBoxClone::box_clone(rules))
                                    } else {
                                        determinerules.resign(epi)
                                    });
                                }
                            }
                        },
                        (VGamePhase::Game(ref mut game), VGamePhaseAction::Game(ref gameaction)) => {
                            handle_err(match gameaction {
                                VGameAction::Stoss => game.stoss(epi),
                                VGameAction::Zugeben(card) => game.zugeben(*card, epi),
                            });
                        },
                        (VGamePhase::GameResult(gameresult), VGamePhaseAction::GameResult(())) => {
                            gameresult.confirm(epi);
                        },
                        (_gamephase, _cmd) => {
                        },
                    };
                }
            }
            while gamephase.which_player_can_do_something().is_none() {
                use VGamePhaseGeneric::*;
                fn next_game(peers: &mut SPeers) -> VGamePhase {
                    /*           E2
                     * E1                      E3
                     *    E0 SN SN-1 ... S1 S0
                     *
                     * E0 E1 E2 E3 [S0 S1 S2 ... SN]
                     * E1 E2 E3 S0 [S1 S2 ... SN E0]
                     * E2 E3 S0 S1 [S2 ... SN E0 E1]
                     */
                    // Players: E0 E1 E2 E3 [S0 S1 S2 ... SN] (S0 is longest waiting inactive player)
                    peers.mapepiopeer.as_raw_mut().rotate_left(1);
                    // Players: E1 E2 E3 E0 [S0 S1 S2 ... SN]
                    if let Some(peer_epi3) = peers.mapepiopeer[EPlayerIndex::EPI3].take() {
                        peers.vecpeer.push(peer_epi3);
                    }
                    // Players: E1 E2 E3 -- [S0 S1 S2 ... SN E0] (E1, E2, E3 may be None)
                    // Fill up players one after another
                    assert!(peers.mapepiopeer[EPlayerIndex::EPI3].is_none());
                    for epi in EPlayerIndex::values() {
                        if peers.mapepiopeer[epi].is_none() && !peers.vecpeer.is_empty() {
                            peers.mapepiopeer[epi] = Some(peers.vecpeer.remove(0));
                        }
                    }
                    // Players: E1 E2 E3 S0 [S1 S2 ... SN E0] (E1, E2, E3 may be None)
                    VGamePhase::DealCards(SDealCards::new(static_ruleset(), peers.n_stock))
                };
                gamephase = match gamephase {
                    DealCards(dealcards) => match dealcards.finish() {
                        Ok(gamepreparations) => GamePreparations(gamepreparations),
                        Err(dealcards) => DealCards(dealcards),
                    },
                    GamePreparations(gamepreparations) => match gamepreparations.finish() {
                        Ok(VGamePreparationsFinish::DetermineRules(determinerules)) => DetermineRules(determinerules),
                        Ok(VGamePreparationsFinish::DirectGame(game)) => Game(game),
                        Ok(VGamePreparationsFinish::Stock(gameresult)) => {
                            let mapepiopeer = &mut self.mapepiopeer;
                            gameresult.apply_payout(&mut self.n_stock, |epi, n_payout| {
                                if let Some(ref mut peer) = mapepiopeer[epi] {
                                    peer.n_money += n_payout;
                                }
                            });
                            next_game(self)
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
                        Ok(gameresult) | Err(gameresult) => {
                            for epi in EPlayerIndex::values() {
                                if let Some(ref mut peer) = self.mapepiopeer[epi] {
                                    peer.n_money += gameresult.an_payout[epi];
                                }
                            }
                            let n_pay_into_stock = -gameresult.an_payout.iter().sum::<isize>();
                            assert!(
                                n_pay_into_stock >= 0 // either pay into stock...
                                || n_pay_into_stock == -self.n_stock // ... or exactly empty it (assume that this is always possible)
                            );
                            self.n_stock += n_pay_into_stock;
                            assert!(0 <= self.n_stock);
                            next_game(self)
                        },
                    },
                };
            }
            if let Some(whichplayercandosomething) = verify!(gamephase.which_player_can_do_something()) {
                fn ask_with_timeout(
                    peer: &mut SPeer,
                    epi: EPlayerIndex,
                    itgamephaseaction: impl Iterator<Item=VGamePhaseAction>,
                    peers_mutex: Arc<Mutex<SPeers>>,
                    gamephaseaction_timeout: VGamePhaseAction,
                ) -> VMessage {
                    let (timerfuture, aborthandle) = future::abortable(STimerFuture::new(
                        /*n_secs*/2,
                        peers_mutex,
                        epi,
                    ));
                    assert!({
                        use std::mem::discriminant;
                        peer.otimeoutcmd.as_ref().map_or(true, |timeoutcmd|
                            discriminant(&timeoutcmd.gamephaseaction)==discriminant(&gamephaseaction_timeout)
                        )
                    }); // only one active timeout cmd
                    peer.otimeoutcmd = Some(STimeoutCmd{
                        gamephaseaction: gamephaseaction_timeout,
                        aborthandle,
                    });
                    task::spawn(timerfuture);
                    VMessage::Ask(itgamephaseaction.collect())
                }
                use VGamePhaseGeneric::*;
                match whichplayercandosomething {
                    DealCards((dealcards, epi_doubling)) => {
                        self.for_each(|oepi, peer| {
                            if Some(epi_doubling)==oepi {(
                                dealcards.first_hand_for(epi_doubling).into(),
                                ask_with_timeout(
                                    peer,
                                    epi_doubling,
                                    [true, false]
                                        .iter()
                                        .map(|b_doubling| 
                                            VGamePhaseAction::DealCards(*b_doubling)
                                        ),
                                    self_mutex.clone(),
                                    VGamePhaseAction::DealCards(/*b_doubling*/false),
                                ),
                            )} else {(
                                vec![],
                                VMessage::Info(format!("Asking {:?} for doubling", epi_doubling)),
                            )}
                        });
                    },
                    GamePreparations((gamepreparations, epi_announce_game)) => {
                        self.for_each(|oepi, peer| {
                            if Some(epi_announce_game)==oepi {
                                let vecgamephaseaction_rules : Vec<_> = allowed_rules(
                                    &gamepreparations.ruleset.avecrulegroup[epi_announce_game],
                                    gamepreparations.fullhand(epi_announce_game),
                                )
                                    .map(|orules|
                                        VGamePhaseAction::GamePreparations(orules.map(TActivelyPlayableRules::to_string))
                                    )
                                    .collect(); // TODO collect needed?
                                let gamephaseaction_rules_default = debug_verify!(vecgamephaseaction_rules.get(0)).unwrap().clone();
                                (
                                    gamepreparations.fullhand(epi_announce_game).get().cards().to_vec(),
                                    ask_with_timeout(
                                        peer,
                                        epi_announce_game,
                                        vecgamephaseaction_rules.into_iter(),
                                        self_mutex.clone(),
                                        gamephaseaction_rules_default,
                                    ),
                                )
                            } else {(
                                vec![],
                                VMessage::Info(format!("Asking {:?} for game", epi_announce_game)),
                            )}
                        });
                    },
                    DetermineRules((determinerules, (epi_determine, vecrulegroup))) => {
                        self.for_each(|oepi, peer| {
                            if Some(epi_determine)==oepi {
                                let vecgamephaseaction_rules : Vec<_> = allowed_rules(
                                    &vecrulegroup,
                                    determinerules.fullhand(epi_determine),
                                )
                                    .map(|orules|
                                        VGamePhaseAction::DetermineRules(orules.map(TActivelyPlayableRules::to_string))
                                    )
                                    .collect(); // TODO collect needed?
                                let gamephaseaction_rules_default = debug_verify!(vecgamephaseaction_rules.get(0)).unwrap().clone();
                                (
                                    determinerules.fullhand(epi_determine).get().cards().to_vec(),
                                    ask_with_timeout(
                                        peer,
                                        epi_determine,
                                        vecgamephaseaction_rules.into_iter(),
                                        self_mutex.clone(),
                                        gamephaseaction_rules_default,
                                    ),
                                )
                            } else {(
                                vec![],
                                VMessage::Info(format!("Re-Asking {:?} for game", epi_determine)),
                            )}
                        });
                    },
                    Game((game, (epi_card, vecepi_stoss))) => {
                        self.for_each(|oepi, peer| {
                            if let Some(epi)=oepi {
                                let mut veccard = Vec::new();
                                if epi_card==epi {
                                    for card in game.ahand[epi_card].cards().iter() {
                                        veccard.push(*card);
                                    }
                                }
                                let mut vecmessage = Vec::new();
                                if vecepi_stoss.contains(&epi) {
                                    vecmessage.push(VGamePhaseAction::Game(VGameAction::Stoss));
                                }
                                if epi_card==epi {
                                    assert!(!veccard.is_empty());
                                    (
                                        veccard.clone(),
                                        ask_with_timeout(
                                            peer,
                                            epi_card,
                                            vecmessage.into_iter(),
                                            self_mutex.clone(),
                                            VGamePhaseAction::Game(VGameAction::Zugeben(
                                                *debug_verify!(game.rules.all_allowed_cards(
                                                    &game.stichseq,
                                                    &game.ahand[epi_card],
                                                ).choose(&mut rand::thread_rng())).unwrap()
                                            )),
                                        ),
                                    )
                                } else {
                                    if veccard.is_empty() && vecmessage.is_empty() {(
                                        veccard, // empty
                                        VMessage::Info(format!("Asking {:?} for card", epi_card))
                                    )} else {(
                                        veccard,
                                        VMessage::Ask(vecmessage)
                                    )}
                                }
                            } else {(
                                vec![],
                                VMessage::Info(format!("Asking {:?} for card", epi_card))
                            )}
                        });
                    },
                    GameResult((_gameresult, mapepib_confirmed)) => {
                        self.for_each(|oepi, peer| {(
                            vec![],
                            match oepi {
                                Some(epi) if !mapepib_confirmed[epi] => {
                                    ask_with_timeout(
                                        peer,
                                        epi,
                                        std::iter::once(VGamePhaseAction::GameResult(())),
                                        self_mutex.clone(),
                                        VGamePhaseAction::GameResult(()),
                                    )
                                },
                                _ => {
                                    VMessage::Info("Game finished".into())
                                },
                            }
                        )});
                    },
                }
            }
            self.ogamephase = Some(gamephase);
            assert!(self.ogamephase.is_some());
        } else {
            self.for_each(|_oepi, _peer| (vec![], VMessage::Info("Waiting for more players.".into())));
        }
    }
}

#[derive(Serialize)]
enum VMessage {
    Info(String),
    Ask(Vec<VGamePhaseAction>),
}

// timer adapted from https://rust-lang.github.io/async-book/02_execution/03_wakeups.html
struct STimerFuture {
    state: Arc<Mutex<STimerFutureState>>,
    peers: Arc<Mutex<SPeers>>,
    epi: EPlayerIndex,
}

struct STimerFutureState {
    b_completed: bool,
    owaker: Option<Waker>,
}

impl Future for STimerFuture {
    type Output = ();
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        if state.b_completed {
            let peers_mutex = self.peers.clone();
            let mut peers = debug_verify!(self.peers.lock()).unwrap();
            if let Some(timeoutcmd) = peers.mapepiopeer[self.epi].as_mut().map(|peer| peer.otimeoutcmd.take()).flatten() {
                peers.send_msg(peers_mutex, Some(self.epi), Some(timeoutcmd.gamephaseaction));
            }
            Poll::Ready(())
        } else {
            state.owaker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl STimerFuture {
    fn new(n_secs: u64, peers: Arc<Mutex<SPeers>>, epi: EPlayerIndex) -> Self {
        let state = Arc::new(Mutex::new(STimerFutureState {
            b_completed: false,
            owaker: None,
        }));
        let thread_shared_state = state.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::new(n_secs, /*nanos*/0));
            let mut state = thread_shared_state.lock().unwrap();
            state.b_completed = true;
            if let Some(waker) = state.owaker.take() {
                waker.wake()
            }
        });
        Self {state, peers, epi}
    }
}

async fn handle_connection(peers: Arc<Mutex<SPeers>>, tcpstream: TcpStream, sockaddr: SocketAddr) {
    println!("Incoming TCP connection from: {}", sockaddr);
    let wsstream = debug_verify!(async_tungstenite::accept_async(tcpstream).await).unwrap();
    println!("WebSocket connection established: {}", sockaddr);
    // Insert the write part of this peer to the peer map.
    let (txmsg, rxmsg) = unbounded();
    let peers_mutex = peers.clone();
    debug_verify!(peers.lock()).unwrap().insert(peers_mutex.clone(), SPeer{
        sockaddr,
        txmsg,
        n_money: 0,
        otimeoutcmd: None,
    });
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
            match serde_json::from_str(str_msg) {
                Ok(gamephaseaction) => peers.send_msg(peers_mutex.clone(), oepi, Some(gamephaseaction)),
                Err(e) => println!("Error: {}", e),
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

