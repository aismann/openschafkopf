use crate::ai::rulespecific::airufspiel::*;
use crate::primitives::*;
use crate::rules::{payoutdecider::*, trumpfdecider::*, *};
use crate::util::*;
use std::{cmp::Ordering, fmt, ops::Add};

#[derive(Clone, Debug)]
pub struct SRulesRufspiel {
    epi : EPlayerIndex,
    efarbe : EFarbe,
    payoutdecider: SPayoutDeciderPointBased<SPointsToWin61>,
}

impl fmt::Display for SRulesRufspiel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rufspiel mit der {}-Sau", self.efarbe)
    }
}

pub type STrumpfDeciderRufspiel = STrumpfDeciderSchlag<
    SStaticSchlagOber, STrumpfDeciderSchlag<
    SStaticSchlagUnter, 
    SStaticFarbeHerz>>;

impl SRulesRufspiel {
    pub fn new(epi: EPlayerIndex, efarbe: EFarbe, payoutparams: SPayoutDeciderParams) -> SRulesRufspiel {
        assert_ne!(efarbe, EFarbe::Herz);
        SRulesRufspiel {
            epi,
            efarbe,
            payoutdecider: SPayoutDeciderPointBased::new(payoutparams, SPointsToWin61{}),
        }
    }

    pub fn rufsau(&self) -> SCard {
        SCard::new(self.efarbe, ESchlag::Ass)
    }

    fn is_ruffarbe(&self, card: SCard) -> bool {
        VTrumpfOrFarbe::Farbe(self.efarbe)==self.trumpforfarbe(card)
    }
}

impl TActivelyPlayableRules for SRulesRufspiel {
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::RufspielLike
    }
}

impl TRulesNoObj for SRulesRufspiel {
    impl_rules_trumpf_noobj!(STrumpfDeciderRufspiel);
}

#[derive(Debug)]
struct SPlayerParties22 {
    aepi_pri: [EPlayerIndex; 2],
}

impl TPlayerParties for SPlayerParties22 {
    fn is_primary_party(&self, epi: EPlayerIndex) -> bool {
        self.aepi_pri[0]==epi || self.aepi_pri[1]==epi
    }
    fn multiplier(&self, _epi: EPlayerIndex) -> isize {
        1
    }
}

impl TRules for SRulesRufspiel {
    impl_rules_trumpf!();

    fn can_be_played(&self, hand: SFullHand) -> bool {
        let it = || {hand.get().cards().iter().filter(|&card| self.is_ruffarbe(*card))};
        it().all(|card| card.schlag()!=ESchlag::Ass)
        && 0<it().count()
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.epi)
    }

    fn stoss_allowed(&self, epi: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool {
        EKurzLang::from_cards_per_player(hand.cards().len());
        assert!(epi!=self.epi || !hand.contains(self.rufsau()));
        (epi==self.epi || hand.contains(self.rufsau())) == (vecstoss.len()%2==1)
    }

    fn payoutinfos(&self, gamefinishedstiche: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SPayoutInfo> {
        let epi_coplayer = debug_verify_eq!(
            rulestatecache.fixed.who_has_card(self.rufsau()),
            unwrap!(gamefinishedstiche.get().completed_stichs().iter()
                .flat_map(|stich| stich.iter())
                .find(|&(_, card)| *card==self.rufsau())
                .map(|(epi, _)| epi))
        );
        assert_ne!(self.epi, epi_coplayer);
        let playerparties = SPlayerParties22{aepi_pri: [self.epi, epi_coplayer]};
        let an_payout_no_stock = &self.payoutdecider.payout(
            self,
            rulestatecache,
            gamefinishedstiche,
            &playerparties,
        );
        assert!(an_payout_no_stock.iter().all(|n_payout_no_stock| 0!=*n_payout_no_stock));
        assert_eq!(an_payout_no_stock[self.epi], an_payout_no_stock[epi_coplayer]);
        assert_eq!(
            an_payout_no_stock.iter()
                .filter(|&n_payout_no_stock| 0<*n_payout_no_stock)
                .count(),
            2
        );
        let estockaction_playerparty = /*b_player_party_wins*/if 0<an_payout_no_stock[self.epi] {
            EStockAction::TakeHalf
        } else {
            EStockAction::GiveHalf
        };
        EPlayerIndex::map_from_fn(|epi|
            SPayoutInfo::new(
                an_payout_no_stock[epi],
                if playerparties.is_primary_party(epi) {estockaction_playerparty} else {EStockAction::Ignore},
            )
        )
    }

    fn payouthints(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SPayoutHint> {
        let epi_coplayer = debug_verify_eq!(
            rulestatecache.fixed.who_has_card(self.rufsau()),
            stichseq.visible_stichs()
                .iter()
                .flat_map(|stich| stich.iter())
                .find(|&(_, card)| *card==self.rufsau())
                .map(|(epi, _)| epi)
                .unwrap_or_else(|| {
                    unwrap!(EPlayerIndex::values().find(|epi|
                        ahand[*epi].cards().iter().any(|card| *card==self.rufsau())
                    ))
                })
        );
        assert_ne!(self.epi, epi_coplayer);
        self.payoutdecider.payouthints(self, stichseq, ahand, rulestatecache, &SPlayerParties22{aepi_pri: [self.epi, epi_coplayer]})
            .map(|pairon_payout| SPayoutHint::new((
                // TODO EStockAction
                pairon_payout.0.map(|n_payout| SPayoutInfo::new(n_payout, EStockAction::Ignore)),
                pairon_payout.1.map(|n_payout| SPayoutInfo::new(n_payout, EStockAction::Ignore)),
            )))
    }

    fn all_allowed_cards_first_in_stich(&self, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        if // do we already know who had the rufsau?
            stichseq.completed_stichs().iter()
                .any(|stich| {
                    assert!(stich.is_full()); // completed_stichs should only process full stichs
                    self.is_ruffarbe(*stich.first()) // gesucht or weggelaufen
                    || stich.iter().any(|(_, card)| *card==self.rufsau()) // We explicitly traverse all cards because it may be allowed (by exotic rules) to schmier rufsau even if not gesucht.
                } )
            // Remark: Player must have 4 cards of ruffarbe on his hand *at this point of time* (i.e. not only at the beginning!)
            || !hand.contains(self.rufsau())
            || 4 <= hand.cards().iter()
                .filter(|&card| self.is_ruffarbe(*card))
                .count()
        {
            hand.cards().clone()
        } else {
            hand.cards().iter()
                .copied()
                .filter(|&card| !self.is_ruffarbe(card) || self.rufsau()==card)
                .collect()
        }
    }

    fn all_allowed_cards_within_stich(&self, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        if hand.cards().len()<=1 {
            hand.cards().clone()
        } else {
            assert!(!stichseq.current_stich().is_empty());
            let epi = unwrap!(stichseq.current_stich().current_playerindex());
            let b_weggelaufen = stichseq.completed_stichs().iter()
                .any(|stich| epi==stich.first_playerindex() && self.is_ruffarbe(*stich.first()));
            let card_first = *stichseq.current_stich().first();
            if self.is_ruffarbe(card_first) && hand.contains(self.rufsau()) && !b_weggelaufen {
                return std::iter::once(self.rufsau()).collect()
            }
            let veccard_allowed : SHandVector = hand.cards().iter().copied()
                .filter(|&card| 
                    self.rufsau()!=card 
                    && self.trumpforfarbe(card)==self.trumpforfarbe(card_first)
                )
                .collect();
            if veccard_allowed.is_empty() {
                if b_weggelaufen {
                    hand.cards().clone()
                } else {
                    hand.cards().iter().copied().filter(|&card| self.rufsau()!=card).collect()
                }
            } else {
                veccard_allowed
            }
        }
    }

    fn rulespecific_ai<'rules>(&'rules self) -> Option<Box<dyn TRuleSpecificAI + 'rules>> {
        Some(Box::new(SAIRufspiel::new(self)))
    }

    fn snapshot_cache(&self, rulestatecachefixed: &SRuleStateCacheFixed) -> Option<Box<dyn TSnapshotCache<SMinMax>>> {
        /*#[derive(Clone, Hash, Eq, PartialEq, Debug, Ord, PartialOrd)]
        struct SSnapshotEquivalenceClass {
            // TODO all this could be represented as one u64, and save memory.
            pointstichcount_primary: SPointStichCount,
            pointstichcount_secondary: SPointStichCount,
            epi_next_stich: EPlayerIndex,
            setcard_played: u32, // TODO enumset
        }*/
        type SSnapshotEquivalenceClass = u64;
        #[derive(Debug)]
        struct SSnapshotCacheRufspiel {
            //rules: &'rules SRulesRufspiel,
            playerparties: SPlayerParties22,
            mapsnapequivpayoutstats: std::collections::HashMap<SSnapshotEquivalenceClass, SMinMax>,
        }
        impl SSnapshotCacheRufspiel {
            fn snap_equiv(&self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache) -> SSnapshotEquivalenceClass {
                //assert_eq!(stichseq.current_stich().size(), 0);
                let point_stich_count = |b_primary| {
                    EPlayerIndex::values()
                        .filter(|epi| b_primary==self.playerparties.is_primary_party(*epi))
                        .map(|epi| rulestatecache.changing.mapepipointstichcount[epi].clone()) // TODO clone needed?
                        .fold(
                            SPointStichCount{n_stich: 0, n_point: 0},
                            SPointStichCount::add,
                        )
                };
                //SSnapshotEquivalenceClass {
                //    pointstichcount_primary: point_stich_count(true),
                //    pointstichcount_secondary: point_stich_count(false),
                //    epi_next_stich: stichseq.current_stich().first_playerindex(),
                //    setcard_played: {
                //        let mut setcard_played = 0;
                //        for (_, &card) in stichseq.visible_stichs().iter().flat_map(SStich::iter) {
                //            let mask = 1 << card.to_usize();
                //            debug_assert_eq!((setcard_played & mask), 0);
                //            setcard_played |= mask;
                //        }
                //        setcard_played
                //    },
                //}
                let pointstichcount_primary = point_stich_count(true);
                let pointstichcount_secondary = point_stich_count(false);
                let epi_next_stich = stichseq.current_stich().first_playerindex();
                let setcard_played = {
                    let mut setcard_played = 0;
                    for (_, &card) in stichseq.visible_stichs().iter().flat_map(SStich::iter) {
                        let mask = 1 << card.to_usize();
                        debug_assert_eq!((setcard_played & mask), 0);
                        setcard_played |= mask;
                    }
                    setcard_played
                };
                // TODO? use bitfield crate
                let mut snapequiv = 0;
                let mut set_bits = |bits| {
                    assert_eq!(snapequiv & bits, 0); // none of the touched bits are set so far
                    snapequiv |= bits;
                };
                set_bits(pointstichcount_primary.n_point.as_num::<u64>() << 0);
                set_bits(pointstichcount_primary.n_stich.as_num::<u64>() << 7);
                set_bits(pointstichcount_secondary.n_point.as_num::<u64>() << 11);
                set_bits(pointstichcount_secondary.n_stich.as_num::<u64>() << 18);
                set_bits(epi_next_stich.to_usize().as_num::<u64>() << 22);
                set_bits(setcard_played << 24);
                snapequiv
            }
        }
        impl TSnapshotCache<SMinMax> for SSnapshotCacheRufspiel {
            fn get(&self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache) -> Option<SMinMax> {
                //println!("SSnapshotCacheRufspiel {:?}", self.mapsnapequivpayoutstats);
                //assert_eq!(stichseq.current_stich().size(), 0);
                self.mapsnapequivpayoutstats
                    .get(&self.snap_equiv(stichseq, rulestatecache))
                    .map(|payoutstats| payoutstats.clone())
            }
            fn put(&mut self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache, payoutstats: &SMinMax) {
                //assert_eq!(stichseq.current_stich().size(), 0);
                self.mapsnapequivpayoutstats
                    .insert(self.snap_equiv(stichseq, rulestatecache), payoutstats.clone());
                //println!("SSnapshotCacheRufspiel {:?}", self.mapsnapequivpayoutstats);
            }
        }
        let epi_coplayer = rulestatecachefixed.who_has_card(self.rufsau());
        let playerparties = SPlayerParties22{aepi_pri: [self.epi, epi_coplayer]};
        Some(Box::new(
            SSnapshotCacheRufspiel{
                playerparties,
                mapsnapequivpayoutstats: Default::default(),
            }
        ))
    }
}
