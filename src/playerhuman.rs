use stich::*;
use card::*;
use hand::*;
use player::*;
use gamestate::*;
use rules::*;
use ruleset::*;
use skui;

use std::sync::mpsc;
use std::io::Read;
use std::rc::Rc;

pub struct CPlayerHuman;

impl CPlayer for CPlayerHuman {
    fn take_control(&mut self, gamestate: &SGameState, txcard: mpsc::Sender<CCard>) {
        let eplayerindex = gamestate.which_player_can_do_something().unwrap();
        skui::print_vecstich(&gamestate.m_vecstich);
        let ref hand = gamestate.m_ahand[eplayerindex];
        skui::print_hand(hand, None);
        txcard.send(
            skui::ask_for_alternative(
                &format!("Your cards: {}", hand),
                &gamestate.m_rules.all_allowed_cards(
                    &gamestate.m_vecstich,
                    &hand
                ),
                |card| card.to_string()
            )
        );
    }

    fn ask_for_game(&self, eplayerindex: EPlayerIndex, hand: &CHand) -> Option<Rc<TRules>> {
        let vecorules = Some(None).into_iter() // TODO is there no singleton iterator?
            .chain(
                ruleset_default(eplayerindex).allowed_rules().iter()
                    .filter(|rules| rules.can_be_played(hand))
                    .map(|rules| Some(rules.clone()))
            )
            .collect();
        skui::print_hand(hand, None);
        skui::ask_for_alternative(
            &format!("Your cards: {}. What do you want to play?", hand),
            &vecorules,
            |orules| match orules {
                &None => "Nothing".to_string(),
                &Some(ref rules) => rules.to_string()
            }
        )
    }
}
