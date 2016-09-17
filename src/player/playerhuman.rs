use primitives::*;
use player::*;
use rules::*;
use rules::ruleset::*;
use game::*;
use skui;
use ai::*;

use std::sync::mpsc;

pub struct SPlayerHuman<'ai> {
    pub m_ai : &'ai TAi,
}

fn choose_ruleset_or_rules<'t, T, FnFormat, FnChoose>(hand: &SHand, vect : &'t Vec<T>, fn_format: FnFormat, fn_choose: FnChoose) -> &'t T
    where FnFormat: Fn(&T) -> String,
          FnChoose: Fn(usize) -> Option<&'t TRules>
{
    &skui::ask_for_alternative(
        vect,
        skui::choose_alternative_from_list_key_bindings(),
        |_ot| {true},
        |ncwin, i_ot_chosen, _ot_suggest| {
            assert!(_ot_suggest.is_none());
            skui::wprintln(ncwin, &format!("Your cards: {}. What do you want to play?", hand));
            for (i_t, t) in vect.iter().enumerate() {
                skui::wprintln(ncwin, &format!("{} {} ({})",
                    if i_t==i_ot_chosen {"*"} else {" "},
                    fn_format(&t),
                    i_t
                ));
            }
            let mut veccard = hand.cards().clone();
            if let Some(rules)=fn_choose(i_ot_chosen) {
                rules.sort_cards_first_trumpf_then_farbe(veccard.as_mut_slice());
            }
            skui::print_hand(&veccard, None);
        },
        || {None}
    )
}

impl<'ai> TPlayer for SPlayerHuman<'ai> {
    fn take_control(&mut self, game: &SGame, txcard: mpsc::Sender<SCard>) {
        skui::print_vecstich(&game.m_vecstich);
        let hand = {
            let mut hand = game.m_ahand[game.which_player_can_do_something().unwrap()].clone();
            game.m_rules.sort_cards_first_trumpf_then_farbe(hand.cards_mut());
            hand
        };
        let veccard_allowed = game.m_rules.all_allowed_cards(&game.m_vecstich, &hand);
        match txcard.send(
            skui::ask_for_alternative(
                &hand.cards(),
                skui::choose_card_from_hand_key_bindings(),
                |card| {veccard_allowed.iter().any(|card_allowed| card_allowed==card)},
                |ncwin, i_card_chosen, ocard_suggest| {
                    if let &Some(card) = ocard_suggest {
                        skui::wprintln(ncwin, &format!("AI: {}", card));
                    }
                    skui::print_hand(hand.cards(), Some(i_card_chosen));
                    skui::print_game_info(game.m_rules, &game.m_vecstoss);
                },
                || {Some(self.m_ai.suggest_card(game))}
            ).clone()
        ) {
            Ok(_) => (),
            Err(_) => unimplemented!(), // we possibly want to be able to deal with "blocked" plays (timeout etc.)
        }
    }

    fn ask_for_game<'rules>(&self, hand: &SHand, vecgameannouncement : &Vec<SGameAnnouncement>, vecrulegroup: &'rules Vec<SRuleGroup>, txorules: mpsc::Sender<Option<&'rules TRules>>) {
        skui::print_game_announcements(vecgameannouncement);
        let vecorulegroup : Vec<Option<&SRuleGroup>> = Some(None).into_iter()
            .chain(
                vecrulegroup.iter()
                    .filter(|rulegroup| rulegroup.m_vecrules.iter()
                        .any(|rules| rules.can_be_played(hand))
                    )
                    .map(|rulegroup| Some(rulegroup))
            )
            .collect();
        while let &Some(rulegroup) = choose_ruleset_or_rules(
            hand,
            &vecorulegroup,
            |orulegroup : &Option<&SRuleGroup>| match orulegroup {
                &None => "Nothing".to_string(),
                &Some(rulegroup) => rulegroup.m_str_name.clone(),
            },
            |i_orulegroup_chosen| vecorulegroup[i_orulegroup_chosen].map(|rulegroup| rulegroup.m_vecrules[0].as_ref().as_rules()),
        )
        {
            let vecorules : Vec<Option<&TRules>> = Some(None).into_iter()
                .chain(
                    rulegroup.m_vecrules.iter()
                        .filter(|rules| rules.can_be_played(hand))
                        .map(|rules| Some(rules.as_ref().as_rules().clone()))
                )
                .collect();
            if let &Some(rules) = choose_ruleset_or_rules(
                hand,
                &vecorules,
                |orules : &Option<&TRules>| match orules {
                    &None => "Back".to_string(),
                    &Some(ref rules) => rules.to_string()
                },
                |i_orules_chosen| vecorules[i_orules_chosen]
            ) {
                txorules.send(Some(rules)).unwrap();
                return;
            }
        }
        txorules.send(None).unwrap();
    }

    fn ask_for_stoss(
        &self,
        _eplayerindex: EPlayerIndex,
        rules: &TRules,
        hand: &SHand,
        vecstoss: &Vec<SStoss>,
        txb: mpsc::Sender<bool>,
    ) {
        let vecb_stoss = vec![false, true];
        txb.send(skui::ask_for_alternative(
            &vecb_stoss,
            skui::choose_alternative_from_list_key_bindings(),
            |_| true, // all alternatives allowed
            |ncwin, i_b_stoss_chosen, ob_stoss_suggest| {
                assert!(ob_stoss_suggest.is_none());
                skui::print_game_info(rules, vecstoss);
                {
                    let mut veccard = hand.cards().clone();
                    rules.sort_cards_first_trumpf_then_farbe(veccard.as_mut_slice());
                    skui::print_hand(&veccard, None);
                }
                for (i_b_stoss, b_stoss) in vecb_stoss.iter().enumerate() {
                    skui::wprintln(ncwin, &format!("{} {} {}",
                        if i_b_stoss==i_b_stoss_chosen {"*"} else {" "},
                        if *b_stoss {"Give"} else {"No"},
                        { match vecstoss.len() {
                            0 => "Kontra",
                            1 => "Re",
                            2 => "Sup",
                            3 => "Hirsch",
                            _ => panic!() // currently only quadruple stoss supported
                        } },
                    ));
                }
            },
            || None, // TODO implement suggestions
        ).clone()).unwrap()
    }
}
