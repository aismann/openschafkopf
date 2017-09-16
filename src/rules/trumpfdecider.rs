use primitives::*;
use rules::*;
use std::cmp::Ordering;
use std::marker::PhantomData;
use util::*;

pub trait TTrumpfDecider : Sync + 'static + Clone + fmt::Debug {
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe;

    fn trumpfs_in_descending_order(veceschlag: Vec<ESchlag>) -> Vec<SCard>;
    fn compare_trumpf(card_fst: SCard, card_snd: SCard) -> Ordering;
    fn count_laufende(gamefinishedstiche: &SGameFinishedStiche, ab_winner: &EnumMap<EPlayerIndex, bool>) -> usize {
        let veccard_trumpf = Self::trumpfs_in_descending_order(Vec::new());
        let mapcardepi = gamefinishedstiche.get().iter()
            .flat_map(|stich| stich.iter())
            .map(|(epi, card)| (*card, epi))
            .collect::<SCardMap<_>>();
        let laufende_relevant = |card: &SCard| {
            ab_winner[mapcardepi[*card]]
        };
        let b_might_have_lauf = laufende_relevant(&veccard_trumpf[0]);
        let ekurzlang = EKurzLang::from_cards_per_player(gamefinishedstiche.get().len());
        veccard_trumpf.iter()
            .filter(|&card| ekurzlang.supports_card(*card))
            .take_while(|card| b_might_have_lauf==laufende_relevant(card))
            .count()
    }
}

#[derive(Clone, Debug)]
pub struct STrumpfDeciderNoTrumpf {}
impl TTrumpfDecider for STrumpfDeciderNoTrumpf {
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe {
        VTrumpfOrFarbe::Farbe(card.farbe())
    }
    fn trumpfs_in_descending_order(mut _veceschlag: Vec<ESchlag>) -> Vec<SCard> {
        Vec::new()
    }
    fn compare_trumpf(_card_fst: SCard, _card_snd: SCard) -> Ordering {
        panic!("STrumpfDeciderNoTrumpf::compare_trumpf called")
    }
}

pub trait TSchlagDesignator : Sync + 'static + Clone + fmt::Debug {const SCHLAG : ESchlag;}
#[derive(Clone, Debug)]
pub struct SSchlagDesignatorOber {}
#[derive(Clone, Debug)]
pub struct SSchlagDesignatorUnter {}
impl TSchlagDesignator for SSchlagDesignatorOber { const SCHLAG : ESchlag = ESchlag::Ober; }
impl TSchlagDesignator for SSchlagDesignatorUnter { const SCHLAG : ESchlag = ESchlag::Unter; }

#[derive(Clone, Debug)]
pub struct STrumpfDeciderSchlag<SchlagDesignator, DeciderSec> {
    schlagdesignator: PhantomData<SchlagDesignator>,
    decidersec: PhantomData<DeciderSec>,
}
impl<SchlagDesignator, DeciderSec> TTrumpfDecider for STrumpfDeciderSchlag<SchlagDesignator, DeciderSec> 
    where DeciderSec: TTrumpfDecider,
          SchlagDesignator: TSchlagDesignator,
{
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe {
        if SchlagDesignator::SCHLAG == card.schlag() {
            VTrumpfOrFarbe::Trumpf
        } else {
            DeciderSec::trumpforfarbe(card)
        }
    }
    fn trumpfs_in_descending_order(mut veceschlag: Vec<ESchlag>) -> Vec<SCard> {
        let mut veccard_trumpf : Vec<_> = EFarbe::values()
            .map(|efarbe| SCard::new(efarbe, SchlagDesignator::SCHLAG))
            .collect();
        veceschlag.push(SchlagDesignator::SCHLAG);
        let mut veccard_trumpf_sec = DeciderSec::trumpfs_in_descending_order(veceschlag);
        veccard_trumpf.append(&mut veccard_trumpf_sec);
        veccard_trumpf
    }
    fn compare_trumpf(card_fst: SCard, card_snd: SCard) -> Ordering {
        match (SchlagDesignator::SCHLAG==card_fst.schlag(), SchlagDesignator::SCHLAG==card_snd.schlag()) {
            (true, true) => {
                // TODORUST static_assert not available in rust, right?
                assert!(EFarbe::Eichel < EFarbe::Gras, "Farb-Sorting can't be used here");
                assert!(EFarbe::Gras < EFarbe::Herz, "Farb-Sorting can't be used here");
                assert!(EFarbe::Herz < EFarbe::Schelln, "Farb-Sorting can't be used here");
                if card_snd.farbe() < card_fst.farbe() {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            },
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => DeciderSec::compare_trumpf(card_fst, card_snd),
        }
    }
}

pub trait TFarbeDesignator : Sync + 'static + Clone + fmt::Debug {const FARBE : EFarbe;}
#[derive(Clone, Debug)]
pub struct SFarbeDesignatorEichel {}
impl TFarbeDesignator for SFarbeDesignatorEichel { const FARBE : EFarbe = EFarbe::Eichel; }
#[derive(Clone, Debug)]
pub struct SFarbeDesignatorGras {}
impl TFarbeDesignator for SFarbeDesignatorGras { const FARBE : EFarbe = EFarbe::Gras; }
#[derive(Clone, Debug)]
pub struct SFarbeDesignatorHerz {}
impl TFarbeDesignator for SFarbeDesignatorHerz { const FARBE : EFarbe = EFarbe::Herz; }
#[derive(Clone, Debug)]
pub struct SFarbeDesignatorSchelln {}
impl TFarbeDesignator for SFarbeDesignatorSchelln { const FARBE : EFarbe = EFarbe::Schelln; }

#[derive(Clone, Debug)]
pub struct STrumpfDeciderFarbe<FarbeDesignator> {
    farbedesignator: PhantomData<FarbeDesignator>,
}
impl<FarbeDesignator> TTrumpfDecider for STrumpfDeciderFarbe<FarbeDesignator> 
    where FarbeDesignator: TFarbeDesignator,
{
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe {
        if FarbeDesignator::FARBE == card.farbe() {
            VTrumpfOrFarbe::Trumpf
        } else {
            VTrumpfOrFarbe::Farbe(card.farbe())
        }
    }
    fn trumpfs_in_descending_order(veceschlag: Vec<ESchlag>) -> Vec<SCard> {
        ESchlag::values()
            .filter(|eschlag| !veceschlag.iter().any(|&eschlag_done| eschlag_done==*eschlag))
            .map(|eschlag| SCard::new(FarbeDesignator::FARBE, eschlag))
            .collect()
    }
    fn compare_trumpf(card_fst: SCard, card_snd: SCard) -> Ordering {
        assert!(Self::trumpforfarbe(card_fst).is_trumpf());
        assert!(Self::trumpforfarbe(card_snd).is_trumpf());
        compare_farbcards_same_color(card_fst, card_snd)
    }
}

macro_rules! impl_rules_trumpf {($trumpfdecider: ident) => {
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe {
        $trumpfdecider::trumpforfarbe(card)
    }
    fn compare_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        $trumpfdecider::compare_trumpf(card_fst, card_snd)
    }
    fn count_laufende(&self, gamefinishedstiche: &SGameFinishedStiche, ab_winner: &EnumMap<EPlayerIndex, bool>) -> usize {
        $trumpfdecider::count_laufende(gamefinishedstiche, ab_winner)
    }
}}
