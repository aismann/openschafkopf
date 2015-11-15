use card::*;
use std::fmt;
use std::cmp::Ordering;

#[derive(Clone)]
pub struct CHand {
    m_veccard: Vec<CCard>, // TODO: use arrayvec
}

impl CHand {
    pub fn new_from_hand(&self, card_played: CCard) -> CHand {
        CHand {
            m_veccard : self
                .m_veccard
                .iter()
                .map(|x| x.clone())
                .filter(|&card| card!=card_played)
                .collect::<Vec<_>>()
        }
    }
    pub fn new_from_vec(veccard: Vec<CCard>) -> CHand {
        CHand {m_veccard : veccard}
    }
    pub fn contains(&self, card_check: CCard) -> bool {
        self.contains_pred(|&card| card==card_check)
    }
    fn contains_pred<Pred>(&self, pred: Pred) -> bool
        where Pred: Fn(&CCard) -> bool
    {
        self.m_veccard
            .iter()
            .any(pred)
    }
    pub fn play_card(&mut self, card_played: CCard) {
        self.m_veccard.retain(|&card| card!=card_played)
    }
    pub fn sort<CmpLess>(&mut self, cmpless: CmpLess)
        where CmpLess: Fn(&CCard, &CCard) -> Ordering
    {
        self.m_veccard.sort_by(cmpless)
    }
    //fn find_best_card_cmp<CmpLess>(&self, cmpless: CmpLess) -> CCard
    //    where CmpLess: Fn(CCard, CCard) -> bool
    //{
    //}
    //fn for_each_card_with<Pred, Func>(&self, pred: Pred, func: Func)
    //    where Pred: Fn(&CCard) -> bool,
    //          Func: Fn(&CCard)
    //{
    //    for card in self.m_veccard.iter().filter(|card| pred(card)) {
    //        func(card);
    //    }
    //}
    //fn count_cards_with<Pred>(&self, pred: Pred) -> usize
    //    where Pred: Fn(&CCard) -> bool
    //{
    //    self.m_veccard.iter().filter(|card| pred(card)).count()
    //}
    pub fn cards(&self) -> &Vec<CCard> {
        &self.m_veccard
    }
}

impl fmt::Display for CHand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for card in self.m_veccard.iter() {
            try!(write!(f, "{}, ", card));
        }
        write!(f, "")
    }
}

#[test]
fn test_hand() {
    let hand = CHand::new_from_vec(
        vec!(
            CCard::new(efarbeEICHEL, eschlagU),
            CCard::new(efarbeHERZ, eschlagK),
            CCard::new(efarbeSCHELLN, eschlag7),
        )
    );
    let hand2 = hand.new_from_hand(CCard::new(efarbeHERZ, eschlagK));
    assert_eq!(hand.cards().len()-1, hand2.cards().len());
    assert!(hand2.cards()[0]==CCard::new(efarbeEICHEL, eschlagU));
    assert!(hand2.cards()[1]==CCard::new(efarbeSCHELLN, eschlag7));
}
