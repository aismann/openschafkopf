use card::*;
use hand::*;
use stich::*;
use rules::*;
use gamestate::*;
use game::*;

use std::sync::mpsc;

pub trait CPlayer {
    fn take_control(&mut self, gamestate: &SGameState, txcard: mpsc::Sender<CCard>);
    // TODO: players need information about who already wants to play
    fn ask_for_game(&self, eplayerindex: EPlayerIndex, hand: &CHand, vecgameannouncement: &Vec<SGameAnnouncement>) -> Option<Box<TRules>>;
}
