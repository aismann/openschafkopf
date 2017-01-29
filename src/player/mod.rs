pub mod playercomputer;
pub mod playerhuman;

use primitives::*;
use rules::*;
use rules::ruleset::*;
use game::*;

use std::sync::mpsc;

pub trait TPlayer {
    fn ask_for_doubling(
        &self,
        veccard: &[SCard],
        txb_doubling: mpsc::Sender<bool>,
    );

    fn ask_for_card(&self, game: &SGame, txcard: mpsc::Sender<SCard>);
    // TODO: players need information about who already wants to play
    fn ask_for_game<'rules>(
        &self,
        hand: &SFullHand,
        gameannouncements: &SGameAnnouncements,
        vecrulegroup: &'rules [SRuleGroup],
        n_stock: isize,
        txorules: mpsc::Sender<Option<&'rules TActivelyPlayableRules>>
    );

    fn ask_for_stoss(
        &self,
        epi: EPlayerIndex,
        doublings: &SDoublings,
        rules: &TRules,
        hand: &SHand,
        vecstoss: &[SStoss],
        n_stock: isize,
        txb: mpsc::Sender<bool>,
    );
}
