extern crate rand;

mod card;
mod stich;
mod combinatorics;
mod cardvectorparser;
mod hand;
mod rules;
mod rulesrufspiel;
mod rulessolo;
mod gamestate;
mod game;
mod player;
mod playercomputer;
mod playerhuman;
mod suspicion;
mod ruleset;

use game::*;
use std::sync::mpsc;
use card::CCard;

fn main() {
    loop {
        let mut game = CGame::new();
        println!("Hand 0 : {}", game.m_gamestate.m_ahand[0]);
        if game.start_game(0) {
            while let Some(eplayerindex)=game.which_player_can_do_something() {
                let (txcard, rxcard) = mpsc::channel::<CCard>();
                game.m_vecplayer[eplayerindex].take_control(
                    &game.m_gamestate,
                    txcard.clone()
                );
                let card_played = rxcard.recv().unwrap();
                game.zugeben(card_played, eplayerindex);
            }
        }
        let an_points = game.points_per_player();
        println!("Results");
        for eplayerindex in 0..4 {
            println!("Player {}: {} points", eplayerindex, an_points[eplayerindex]);
        }
    }
}
