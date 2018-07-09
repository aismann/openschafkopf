use game::*;

fn hand_to_vecefarbeeschlag(hand: &SHand) -> Vec<(EFarbe, ESchlag)> {
    hand.cards().iter().map(|card| (card.farbe(), card.schlag())).collect()
}


#[derive(Debug)]
pub enum VGamePhase<'rules> {
    DealCards(SDealCards<'rules>),
    GamePreparations(SGamePreparations<'rules>),
    DetermineRules(SDetermineRules<'rules>),
    Game(SGame),
    GameResult(SGameResult),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum VGameCommand {
    AnnounceDoubling(EPlayerIndex, bool),
    AnnounceGame(EPlayerIndex, Option<SActivelyPlayableRulesID>),
    Stoss(EPlayerIndex, bool),
    Zugeben(EPlayerIndex, SCard),
}

type SWebsocketVecCard = Vec<(EFarbe, ESchlag)>;

#[derive(new, Serialize)]
pub struct SDealCardsPublicInfo {
    hand : SWebsocketVecCard,
    doublings : SDoublings,
    n_stock : isize,
}

#[derive(new, Serialize)]
pub struct SGamePreparationsPublicInfo {
    hand : SWebsocketVecCard,
    doublings : SDoublings,
    gameannouncementsstr : SGenericGameAnnouncements<String>,
    n_stock : isize,
}

#[derive(new, Serialize)]
pub struct SDetermineRulesPublicInfo {
    hand : SWebsocketVecCard,
    doublings : SDoublings,
    vecpairepistr_queued : Vec<(EPlayerIndex, String)>,
    n_stock : isize,
    pairepistr_current_bid : (EPlayerIndex, String),
}

#[derive(new, Serialize)]
pub struct SGamePublicInfo {
    hand : SWebsocketVecCard,
    doublings : SDoublings,
    str_rules : String,
    vecstoss : Vec<SStoss>,
    ostossparams : Option<SStossParams>,
    n_stock : isize,
    vecstich : Vec<SStich>,
}

#[derive(Serialize)]
pub enum VGamePublicInfo {
    DealCards(SDealCardsPublicInfo),
    GamePreparations(SGamePreparationsPublicInfo),
    DetermineRules(SDetermineRulesPublicInfo),
    Game(SGamePublicInfo),
    GameResult(SGameResult),
}

impl<'rules> VGamePhase<'rules> {
    pub fn publicinfo(&self, epi: EPlayerIndex) -> (VGamePublicInfo, Vec<(String, VGameCommand)>) {
        fn publicinfo_allowedrules(
            epi_active: EPlayerIndex,
            vecrulegroup: &[SRuleGroup],
            hand: &SHand,
            ekurzlang: EKurzLang,
        ) -> Vec<(String, VGameCommand)> {
            allowed_rules(vecrulegroup)
                .filter(|orules| orules.map_or(/*allow playing nothing*/true, |rules|
                    rules.can_be_played(SFullHand::new(hand, ekurzlang))
                ))
                .map(|orules| match orules {
                    None => (
                        "Nothing".to_string(),
                        VGameCommand::AnnounceGame(epi_active, None)
                    ),
                    Some(rules) => (
                        format!("{}", rules),
                        VGameCommand::AnnounceGame(epi_active, Some(rules.rulesid())),
                    ),
                })
                .collect()
        };
        match self {
            VGamePhase::DealCards(dealcards) => {
                (
                    VGamePublicInfo::DealCards(SDealCardsPublicInfo::new(
                        hand_to_vecefarbeeschlag(&dealcards.ahand[epi]),
                        dealcards.doublings.clone(), dealcards.n_stock
                    )),
                    dealcards.which_player_can_do_something()
                        .filter(|epi_active| epi==*epi_active)
                        .map(|epi_active| {
                            [("Doubling", true), ("No doubling", false)].into_iter()
                                .map(|&(str_doubling, b_doubling)| (
                                    str_doubling.to_owned(),
                                    VGameCommand::AnnounceDoubling(epi_active, b_doubling)
                                ))
                                .collect()
                        })
                        .unwrap_or_default(),
                )
            },
            VGamePhase::GamePreparations(gamepreparations) => {
                (
                    VGamePublicInfo::GamePreparations(SGamePreparationsPublicInfo::new(
                        hand_to_vecefarbeeschlag(&gamepreparations.ahand[epi]),
                        gamepreparations.doublings.clone(),
                        gamepreparations.gameannouncements.map(|orules|
                            orules.as_ref().map(|rules| rules.to_string())
                        ),
                        gamepreparations.n_stock,
                    )),
                    gamepreparations.which_player_can_do_something()
                        .filter(|epi_active| epi==*epi_active)
                        .map(|epi_active|
                             publicinfo_allowedrules(
                                 epi_active,
                                 &gamepreparations.ruleset.avecrulegroup[epi_active],
                                 &gamepreparations.ahand[epi_active],
                                 gamepreparations.ruleset.ekurzlang,
                             )
                        )
                        .unwrap_or_default(),
                )
            },
            VGamePhase::DetermineRules(determinerules) => {
                (
                    VGamePublicInfo::DetermineRules(SDetermineRulesPublicInfo::new(
                        hand_to_vecefarbeeschlag(&determinerules.ahand[epi]),
                        determinerules.doublings.clone(),
                        determinerules.vecpairepirules_queued.iter()
                            .map(|pairepirules| (
                                pairepirules.0,
                                pairepirules.1.to_string(),
                            ))
                            .collect(),
                        determinerules.n_stock,
                        (
                            determinerules.pairepirules_current_bid.0,
                            determinerules.pairepirules_current_bid.1.to_string(),
                        ),
                    )),
                    determinerules.which_player_can_do_something()
                        .filter(|tplepivecrulegroup| tplepivecrulegroup.0==epi)
                        .map(|tplepivecrulegroup|
                            publicinfo_allowedrules(
                                tplepivecrulegroup.0,
                                &tplepivecrulegroup.1,
                                &determinerules.ahand[tplepivecrulegroup.0],
                                determinerules.ruleset.ekurzlang,
                            )
                        )
                        .unwrap_or_default(),
                )
            },
            VGamePhase::Game(game) => {
                (
                    VGamePublicInfo::Game(SGamePublicInfo::new(
                        hand_to_vecefarbeeschlag(&game.ahand[epi]),
                        game.doublings.clone(),
                        game.rules.to_string(),
                        game.vecstoss.clone(),
                        game.ostossparams.clone(),
                        game.n_stock,
                        game.vecstich.clone(),
                    )),
                    {
                        let mut vectplstrgamecmd = Vec::new();
                        if let Some(gameaction) = game.which_player_can_do_something() {
                            if epi==gameaction.0 {
                                for card in game.rules.all_allowed_cards(&game.vecstich, &game.ahand[epi]) {
                                    vectplstrgamecmd.push((
                                        format!("{}", card),
                                        VGameCommand::Zugeben(epi, card),
                                    ));
                                }
                            }
                            if gameaction.1.contains(&epi) {
                                vectplstrgamecmd.push((
                                    "Stoss".to_owned(),
                                    VGameCommand::Stoss(epi, true)
                                ));
                            }
                        }
                        vectplstrgamecmd
                    },
                )
            },
            VGamePhase::GameResult(gameresult) => {
                (
                    VGamePublicInfo::GameResult(gameresult.clone()),
                    Vec::new(), // do not offer any actions for finished game
                )
            },
        }
    }

    fn finish_phase<GamePhase, FnOk, FnErr>(
        gamephase: GamePhase,
        fn_ok: FnOk,
        fn_err: FnErr,
    ) -> Result<Self, Self>
        where
            GamePhase: TGamePhase,
            FnOk: FnOnce(GamePhase::Finish) -> Self,
            FnErr: FnOnce(GamePhase) -> Self,
    {
        gamephase.finish().map(fn_ok).map_err(fn_err)
    }

    pub fn command(self, gamecmd: VGameCommand) -> Result<Self, Self> {
        match self {
            VGamePhase::DealCards(mut dealcards) => {
                if let VGameCommand::AnnounceDoubling(epi, b_doubling) = gamecmd {
                    dealcards.announce_doubling(epi, b_doubling).ok();
                }
                Self::finish_phase(
                    dealcards,
                    VGamePhase::GamePreparations,
                    VGamePhase::DealCards,
                )
            },
            VGamePhase::GamePreparations(mut gamepreparations) => {
                if let VGameCommand::AnnounceGame(epi, orulesid) = gamecmd {
                    if let Some(orules) = gamepreparations.ruleset.actively_playable_rules_by_id(epi, &orulesid) {
                        assert_eq!(Some(epi), orules.as_ref().map_or(Some(epi), |rules| rules.playerindex()));
                        gamepreparations.announce_game(epi, orules).ok();
                    }
                }
                Self::finish_phase(
                    gamepreparations,
                    |gamepreparationsfinish| {
                        match gamepreparationsfinish {
                            VGamePreparationsFinish::DetermineRules(determinerules) => {
                                VGamePhase::DetermineRules(determinerules)
                            },
                            VGamePreparationsFinish::DirectGame(game) => {
                                VGamePhase::Game(game)
                            },
                            VGamePreparationsFinish::Stock(_n_stock) => {
                                unimplemented!()
                            }
                        }

                    },
                    VGamePhase::GamePreparations,
                )
            },
            VGamePhase::DetermineRules(mut determinerules) => {
                if let VGameCommand::AnnounceGame(epi, orulesid) = gamecmd {
                    if let Some(orules) = determinerules.ruleset.actively_playable_rules_by_id(epi, &orulesid) {
                        match orules {
                            None => determinerules.resign(epi).ok(),
                            Some(rules) => {
                                assert_eq!(Some(epi), rules.playerindex());
                                determinerules.announce_game(epi, rules).ok()
                            }
                        };
                    }
                }
                Self::finish_phase(
                    determinerules,
                    VGamePhase::Game,
                    VGamePhase::DetermineRules,
                )
            },
            VGamePhase::Game(mut game) => {
                match gamecmd {
                    VGameCommand::Stoss(epi, b_stoss) if b_stoss => game.stoss(epi).ok(),
                    VGameCommand::Zugeben(epi, card) => game.zugeben(card, epi).ok(),
                    _ => Some(()),
                };
                Self::finish_phase(
                    game,
                    VGamePhase::GameResult,
                    VGamePhase::Game,
                )
            },
            VGamePhase::GameResult(gameresult) => {
                Err(VGamePhase::GameResult(gameresult))
            },
        }
    }
}
