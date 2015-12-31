use stich::*;
use card::*;
use hand::*;
use player::*;
use gamestate::*;
use rules::*;
use rulesrufspiel::*;
use ruleset::*;

use std::sync::mpsc;
use std::io::{self, Read};
use std::fmt::Display;
use std::rc::Rc;

pub struct CPlayerHuman;

fn ask_for_alternative<T, FnFormat>(vect: &Vec<T>, fn_format: FnFormat) -> T 
    where T : Clone,
          FnFormat : Fn(&T) -> String
{
    assert!(0<vect.len());
    if 1==vect.len() {
        return vect[0].clone(); // just return if there's no choice anyway
    }
    println!("Please choose:");
    loop {
        for (i_t, t) in vect.iter().enumerate() {
            println!("{} ({})", fn_format(&t), i_t);
        }
        let mut str_index = String::new();
        if let Err(e) = (io::stdin().read_line(&mut str_index)) {
            return vect[0].clone(); // TODO: make return type optional?
        }
        match str_index.trim().parse::<usize>() {
            Ok(i) if i < vect.len() => {
                return vect[i].clone();
            }
            Ok(_) => {
                println!("Error. Number not within suggested bounds.");
            }
            _ => {
                println!("Error. Input not a number");
            }
        }
    }
}

impl CPlayer for CPlayerHuman {
    fn take_control(&mut self, gamestate: &SGameState, txcard: mpsc::Sender<CCard>) {
        let eplayerindex = gamestate.which_player_can_do_something().unwrap();
        for (i_stich, stich) in gamestate.m_vecstich.iter().enumerate() {
            println!("Stich {}: {}", i_stich, stich);
        }
        println!("Your cards: {}", gamestate.m_ahand[eplayerindex]);
        txcard.send(
            ask_for_alternative(
                &gamestate.m_rules.all_allowed_cards(
                    &gamestate.m_vecstich,
                    &gamestate.m_ahand[eplayerindex]
                ),
                |card| card.to_string()
            )
        );
    }

    fn ask_for_game(&self, eplayerindex: EPlayerIndex, hand: &CHand) -> Option<Rc<TRules>> {
        let vecorules = Some(None).into_iter() // TODO is there no singleton iterator?
            .chain(
                ruleset_default(eplayerindex).m_vecrules.iter()
                    .filter(|rules| rules.can_be_played(hand))
                    .map(|rules| Some(rules.clone()))
            )
            .collect();
        ask_for_alternative(
            &vecorules,
            |orules| match orules {
                &None => "Nothing".to_string(),
                &Some(ref rules) => rules.to_string()
            }
        )
    }
}
