use crate::ai::{*, handiterators::*, suspicion::*};
use crate::game::SStichSequence;
use crate::primitives::*;
use crate::util::*;
use crate::rules::*;
use crate::cardvector::*;
use itertools::*;
use combine::{char::*, *};
use fxhash::FxHashSet as HashSet;
use rand::prelude::*;
use arrayvec::ArrayVec;

plain_enum_mod!(moderemainingcards, ERemainingCards {_1, _2, _3, _4, _5, _6, _7, _8,});
plain_enum_mod!(modecardindex, ECardIndex {_1, _2, _3, _4, _5, _6, _7, _8,});

type SWinnerIndexCacheArr<T> = EnumMap<EPlayerIndex, EnumMap<ECardIndex, EnumMap<ECardIndex, EnumMap<ECardIndex, EnumMap<ECardIndex, T>>>>>; // TODO "8" should be unwrap!(EPlayerIndex::values().map(|ekurzlang| ekurzlang.cards_per_player()).max())

struct SWinnerIndexCache {
    mapcardecardindex: EnumMap<SCard, ECardIndex>,
    aaaaepi: SWinnerIndexCacheArr<EPlayerIndex>,
    //if_dbg_else!({aaaab: SWinnerIndexCacheArr<bool>}{}),
}

impl SWinnerIndexCache {
    fn new(ahand: &EnumMap<EPlayerIndex, SHand>, rules: &dyn TRules) -> Self {
        let aaaaepi = EPlayerIndex::map_from_fn(|_| ECardIndex::map_from_fn(|_| ECardIndex::map_from_fn(|_| ECardIndex::map_from_fn(|_| ECardIndex::map_from_fn(|_| EPlayerIndex::EPI0)))));
        let mut mapcardecardindex = SCard::map_from_fn(|_| ECardIndex::_1);
        //if_dbg_else!(let mut aaaab = Default::default());
        for epi in EPlayerIndex::values() {
            for (ecardindex, card) in ECardIndex::values().zip(ahand[epi].cards().iter()) {
                mapcardecardindex[*card] = ecardindex;
            }
        }
        let mut slf = Self {
            aaaaepi,
            mapcardecardindex,
            //if_dbg_else!({aaaab,}{})
        };
        for epi in EPlayerIndex::values() {
            for card_0 in ahand[epi.wrapping_add(0)].cards().iter().copied() {
                for card_1 in ahand[epi.wrapping_add(1)].cards().iter().copied() {
                    for card_2 in ahand[epi.wrapping_add(2)].cards().iter().copied() {
                        for card_3 in ahand[epi.wrapping_add(3)].cards().iter().copied() {
                            let stich = SStich::new_full(epi, [card_0, card_1, card_2, card_3]);
                            let slccard = stich.elements_in_order();
                            slf.aaaaepi[epi][slf.mapcardecardindex[slccard[0]]][slf.mapcardecardindex[slccard[1]]][slf.mapcardecardindex[slccard[2]]][slf.mapcardecardindex[slccard[3]]] = rules.winner_index(&stich);
                            //if_dbg_else!(aaaab[card_0][card_1][card_2][card_3]=true);
                            debug_assert_eq!(rules.winner_index(&stich), slf.get(&stich));
                        }
                    }
                }
            }
        }
        slf
    }
    fn get(&self, stich: &SStich) -> EPlayerIndex {
        let slccard = stich.elements_in_order();
        self.aaaaepi[stich.first_playerindex()][self.mapcardecardindex[slccard[0]]][self.mapcardecardindex[slccard[1]]][self.mapcardecardindex[slccard[2]]][self.mapcardecardindex[slccard[3]]]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum VNumVal {
    Const(usize),
    Card(SCard, EPlayerIndex),
    TrumpfOrFarbe(VTrumpfOrFarbe, EPlayerIndex),
    Schlag(ESchlag, EPlayerIndex),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum VConstraint {
    Not(Box<VConstraint>),
    Relation {
        numval_lhs: VNumVal,
        ord: std::cmp::Ordering,
        numval_rhs: VNumVal,
    },
    Conjunction(Box<VConstraint>, Box<VConstraint>),
    Disjunction(Box<VConstraint>, Box<VConstraint>),
}

impl VNumVal {
    fn eval(&self, ahand: &EnumMap<EPlayerIndex, SHand>, rules: &dyn TRules) -> usize {
        fn count(hand: &SHand, fn_pred: impl Fn(&SCard)->bool) -> usize {
            hand.cards().iter().copied().filter(fn_pred).count()
        }
        match self {
            VNumVal::Const(n) => *n,
            VNumVal::Card(card, epi) => count(&ahand[*epi], |card_hand| card_hand==card),
            VNumVal::TrumpfOrFarbe(trumpforfarbe, epi) => count(&ahand[*epi], |card|
                trumpforfarbe==&rules.trumpforfarbe(*card)
            ),
            VNumVal::Schlag(eschlag, epi) => count(&ahand[*epi], |card|
                card.schlag()==*eschlag
            ),
        }
    }
}

impl std::fmt::Display for VNumVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            VNumVal::Const(n) => write!(f, "{}", n),
            VNumVal::Card(card, epi) => write!(f, "{}({})", card, epi),
            VNumVal::TrumpfOrFarbe(trumpforfarbe, epi) => match trumpforfarbe {
                VTrumpfOrFarbe::Trumpf => write!(f, "t({})", epi),
                VTrumpfOrFarbe::Farbe(efarbe) => write!(f, "{}({})", efarbe, epi),
            },
            VNumVal::Schlag(eschlag, epi) => write!(f, "{}({})", eschlag, epi),
        }
    }
}

impl VConstraint {
    fn eval(&self, ahand: &EnumMap<EPlayerIndex, SHand>, rules: &dyn TRules) -> bool {
        match self {
            VConstraint::Not(constraint) => !constraint.eval(ahand, rules),
            VConstraint::Relation{numval_lhs, ord, numval_rhs} => *ord == numval_lhs.eval(ahand, rules).cmp(&numval_rhs.eval(ahand, rules)),
            VConstraint::Conjunction(constraint_lhs, constraint_rhs) => constraint_lhs.eval(ahand, rules) && constraint_rhs.eval(ahand, rules),
            VConstraint::Disjunction(constraint_lhs, constraint_rhs) => constraint_lhs.eval(ahand, rules) || constraint_rhs.eval(ahand, rules),
        }
    }
}

impl std::fmt::Display for VConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            VConstraint::Not(constraint) => write!(f, "!({})", constraint),
            VConstraint::Relation{numval_lhs, ord, numval_rhs} => write!(f, "({}){}({})",
                numval_lhs,
                match ord {
                    std::cmp::Ordering::Less => "<",
                    std::cmp::Ordering::Equal => "=",
                    std::cmp::Ordering::Greater => ">",
                },
                numval_rhs
            ),
            VConstraint::Conjunction(constraint_lhs, constraint_rhs) => write!(f, "({})&({})", constraint_lhs, constraint_rhs),
            VConstraint::Disjunction(constraint_lhs, constraint_rhs) => write!(f, "({})|({})", constraint_lhs, constraint_rhs),
        }
    }
}

fn numval_parser<I: Stream<Item=char>>() -> impl Parser<Input = I, Output = VNumVal>
    where I::Error: ParseError<I::Item, I::Range, I::Position>, // Necessary due to rust-lang/rust#24159
{
    pub fn epi_parser<I: Stream<Item=char>>() -> impl Parser<Input = I, Output = EPlayerIndex>
        where I::Error: ParseError<I::Item, I::Range, I::Position>, // Necessary due to rust-lang/rust#24159
    {
        (spaces(), char('('), spaces())
            .with(choice!(
                char('0').map(|_chr| EPlayerIndex::EPI0),
                char('1').map(|_chr| EPlayerIndex::EPI1),
                char('2').map(|_chr| EPlayerIndex::EPI2),
                char('3').map(|_chr| EPlayerIndex::EPI3)
            ))
            .skip((spaces(), char(')'), spaces()))
    }
    choice!(
        attempt((card_parser(), epi_parser()).map(|(card, epi)| VNumVal::Card(card, epi))),
        (
            choice!(
                choice!(char('t'), char('T')).map(|_| VTrumpfOrFarbe::Trumpf),
                farbe_parser().map(VTrumpfOrFarbe::Farbe)
            ),
            epi_parser()
        ).map(|(trumpforfarbe, epi)| VNumVal::TrumpfOrFarbe(trumpforfarbe, epi)),
        attempt((schlag_parser(), epi_parser()).map(|(eschlag, epi)| VNumVal::Schlag(eschlag, epi))),
        (many1(digit())./*TODO use and_then and get rid of unwrap*/map(|string: /*TODO String needed?*/String|
            unwrap!(string.parse::<usize>())
        )).map(VNumVal::Const)
    )
}

fn single_constraint_parser_<I: Stream<Item=char>>() -> impl Parser<Input = I, Output = VConstraint>
    where I::Error: ParseError<I::Item, I::Range, I::Position>, // Necessary due to rust-lang/rust#24159
{
    choice!(
        (char('!').with(single_constraint_parser())).map(|constraint| VConstraint::Not(Box::new(constraint))),
        char('(').with(constraint_parser()).skip(char(')')),
        (
            numval_parser(),
            optional((
                choice!(
                    char('<').map(|_chr| std::cmp::Ordering::Less),
                    char('=').map(|_chr| std::cmp::Ordering::Equal),
                    char('>').map(|_chr| std::cmp::Ordering::Greater)
                ),
                numval_parser()
            ))
        ).map(|(numval_lhs, otplordnumval_rhs)| {
            let (ord, numval_rhs) = otplordnumval_rhs.unwrap_or((
                std::cmp::Ordering::Greater,
                VNumVal::Const(0)
            ));
            VConstraint::Relation{numval_lhs, ord, numval_rhs}
        })
    )
}
parser!{
    fn single_constraint_parser[I]()(I) -> VConstraint
        where [I: Stream<Item = char>]
    {
        single_constraint_parser_()
    }
}

fn constraint_parser_<I: Stream<Item=char>>() -> impl Parser<Input = I, Output = VConstraint>
    where I::Error: ParseError<I::Item, I::Range, I::Position>, // Necessary due to rust-lang/rust#24159
{
    choice!(
        attempt((sep_by1::<Vec<_>,_,_>(single_constraint_parser(), (spaces(), char('&'), spaces())))
            .map(|vecconstraint| unwrap!(vecconstraint.into_iter().fold1(|constraint_lhs, constraint_rhs|
                VConstraint::Conjunction(Box::new(constraint_lhs), Box::new(constraint_rhs))
            )))),
        attempt((sep_by1::<Vec<_>,_,_>(single_constraint_parser(), (spaces(), char('|'), spaces())))
            .map(|vecconstraint| unwrap!(vecconstraint.into_iter().fold1(|constraint_lhs, constraint_rhs|
                VConstraint::Disjunction(Box::new(constraint_lhs), Box::new(constraint_rhs))
            )))),
        attempt(single_constraint_parser())
    )
}

parser!{
    fn constraint_parser[I]()(I) -> VConstraint
        where [I: Stream<Item = char>]
    {
        constraint_parser_()
    }
}

#[test]
fn test_constraint_parser() {
    fn test_internal(str_in: &str, constraint: VConstraint) {
        assert_eq!(str_in.parse(), Ok(constraint));
    }
    use VConstraint::*;
    use VNumVal::*;
    use EFarbe::*;
    use ESchlag::*;
    use EPlayerIndex::*;
    use VTrumpfOrFarbe::*;
    use std::cmp::Ordering::*;
    fn test_comparison(str_in: &str, numval_lhs: VNumVal, ord: std::cmp::Ordering, numval_rhs: VNumVal) {
        let relation = Relation{numval_lhs, ord, numval_rhs};
        test_internal(str_in, relation.clone());
        test_internal(&format!("!{}", str_in), Not(Box::new(relation.clone())));
        test_internal(&format!("!!{}", str_in), Not(Box::new(Not(Box::new(relation)))));
    }
    fn test_simple_greater_0(str_in: &str, numval_lhs: VNumVal) {
        test_comparison(str_in, numval_lhs, Greater, Const(0));
    }
    test_simple_greater_0("ea(1)", Card(SCard::new(Eichel, Ass), EPI1));
    test_simple_greater_0("t(2)", TrumpfOrFarbe(Trumpf, EPI2));
    test_simple_greater_0("e(0)", TrumpfOrFarbe(Farbe(Eichel), EPI0));
    test_simple_greater_0("o(0)", Schlag(Ober, EPI0));
    test_simple_greater_0("7(0)", Schlag(S7, EPI0));
    test_simple_greater_0("7", Const(7));
    test_comparison("ea(1)>e(0)", Card(SCard::new(Eichel, Ass), EPI1), Greater, TrumpfOrFarbe(Farbe(Eichel), EPI0));
    test_comparison("t(2)=t(3)", TrumpfOrFarbe(Trumpf, EPI2), Equal, TrumpfOrFarbe(Trumpf, EPI3));
    test_comparison("e(0)>3", TrumpfOrFarbe(Farbe(Eichel), EPI0), Greater, Const(3));
    test_comparison("o(0)<3", Schlag(Ober, EPI0), Less, Const(3));
    test_comparison("8(0)<2", Schlag(S8, EPI0), Less, Const(2));
    test_comparison("8<2", Const(8), Less, Const(2));
    // TODO more tests
}

impl std::str::FromStr for VConstraint {
    type Err = (); // TODO? better type
    fn from_str(str_in: &str) -> Result<Self, Self::Err> {
        spaces()
            .with(constraint_parser())
            .skip(spaces())
            .skip(eof())
            // end of parser
            .parse(str_in)
            .map_err(|_| ())
            .map(|pairoutconsumed| pairoutconsumed.0)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct SSetCard {
    n: usize,
}
impl SSetCard {
    fn new() -> Self {
        Self {n:0}
    }
    fn play(&mut self, card: SCard) {
        self.n |= 1<<card.to_usize();
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct SBeginning {
    setcard: SSetCard,
    pointstichcount_epi0: SPointStichCount,
    pointstichcount_other: SPointStichCount,
    epi_current: EPlayerIndex,
}
impl SBeginning {
    fn new(setcard: SSetCard, stichseq: &SStichSequence, rulestatecachechanging: &SRuleStateCacheChanging) -> Self {
        Self {
            setcard,
            pointstichcount_epi0: rulestatecachechanging.mapepipointstichcount[EPlayerIndex::EPI0].clone(),
            pointstichcount_other: SPointStichCount {
                n_stich:
                    rulestatecachechanging.mapepipointstichcount[EPlayerIndex::EPI1].n_stich
                    + rulestatecachechanging.mapepipointstichcount[EPlayerIndex::EPI2].n_stich
                    + rulestatecachechanging.mapepipointstichcount[EPlayerIndex::EPI3].n_stich,
                n_point:
                    rulestatecachechanging.mapepipointstichcount[EPlayerIndex::EPI1].n_point
                    + rulestatecachechanging.mapepipointstichcount[EPlayerIndex::EPI2].n_point
                    + rulestatecachechanging.mapepipointstichcount[EPlayerIndex::EPI3].n_point,
            },
            epi_current: stichseq.current_stich().first_playerindex(),
        }
    }
}

struct SEquivalenceLists {
    mapcardcard_next: EnumMap<SCard, SCard>,
    mapcardcard_prev: EnumMap<SCard, SCard>,
}
impl SEquivalenceLists {
    fn new(slcslccard: &[&[SCard]], stichseq: &SStichSequence) -> Self {
        let mut mapcardcard_next = SCard::map_from_fn(|card| card);
        let mut mapcardcard_prev = SCard::map_from_fn(|card| card);
        for slccard in slcslccard {
            for (card_from, card_to) in slccard.iter().tuple_windows() {
                mapcardcard_next[*card_from] = *card_to;
                mapcardcard_prev[*card_to] = *card_from;
            }
        }
        let mut slf = Self {
            mapcardcard_next,
            mapcardcard_prev,
        };
        for slccard in slcslccard {
            let mut itcard = slccard.iter().copied();
            if let Some(mut card) = itcard.next() {
                assert!(slf.prev(card).is_none());
                for card_next in itcard {
                    assert_eq!(unwrap!(slf.next(card)), card_next);
                    assert_eq!(unwrap!(slf.prev(card_next)), card);
                    card = card_next;
                }
                assert!(slf.next(card).is_none());
            }
            assert!(slccard.iter().copied().skip(1).eq(slf.nexts(slccard[0])));
            assert!(slccard.iter().copied().rev().skip(1).eq(slf.prevs(*unwrap!(slccard.last()))));
        }
        for stich in stichseq.completed_stichs().iter() {
            for (_epi, card_played) in stich.iter() {
                slf.remove(*card_played);
            }
        }
        slf
    }
    fn next(&self, card: SCard) -> Option<SCard> {
        let card_next = self.mapcardcard_next[card];
        if_then_some!(card_next!=card, card_next)
    }
    fn prev(&self, card: SCard) -> Option<SCard> {
        let card_prev = self.mapcardcard_prev[card];
        if_then_some!(card_prev!=card, card_prev)
    }
    fn nexts<'slf>(&'slf self, card: SCard) -> impl Iterator<Item=SCard> + 'slf {
        std::iter::successors(self.next(card), move |&card| self.next(card))
    }
    fn prevs<'slf>(&'slf self, card: SCard) -> impl Iterator<Item=SCard> + 'slf {
        std::iter::successors(self.prev(card), move |&card| self.prev(card))
    }
    fn remove(&mut self, card: SCard) {
        let ocard_next = self.next(card);
        let ocard_prev = self.prev(card);
        if let Some(card_next) = ocard_next.as_ref() {
            self.mapcardcard_prev[*card_next] = if let Some(card_prev) = ocard_prev.as_ref() {
                *card_prev
            } else {
                *card_next
            };
            assert_eq!(self.prev(*card_next), ocard_prev);
        }
        if let Some(card_prev) = ocard_prev.as_ref() {
            self.mapcardcard_next[*card_prev] = if let Some(card_next) = ocard_next.as_ref() {
                *card_next
            } else {
                *card_prev
            };
            assert_eq!(self.next(*card_prev), ocard_next);
        }
    }
    // fn remove_and_undo<R>(&mut self, card: SCard, f: impl FnOnce(&mut Self)->R) -> R {
    //     let card_next = self.mapcardcard_next[card];
    //     let card_prev = self.mapcardcard_prev[card];
    //     self.mapcardcard_prev[card_next] = card_prev;
    //     self.mapcardcard_next[card_prev] = card_next;
    //     self.remove(card);
    //     let r = f(self);
    //     self.mapcardcard_next[card_prev] = card;
    //     self.mapcardcard_prev[card_next] = card;
    //     r
    // }
}

trait TVecExt<T> {
    fn retain_from_to_end(&mut self, i_start: usize, fn_filter: impl Fn(&T)->bool);
    fn find_remove(&mut self, t: &T) -> bool where T: Eq;
}

macro_rules! impl_vecext{($t:ty) => {
    impl<T> TVecExt<T> for $t {
        fn retain_from_to_end(&mut self, i_start: usize, fn_filter: impl Fn(&T)->bool) {
            // adapted from Vec::retain
            let len = self.len();
            let mut del = 0;
            {
                let v = &mut **self;
                for i in i_start..len {
                    if !fn_filter(&v[i]) {
                        del += 1;
                    } else if del > 0 {
                        v.swap(i - del, i); // TODO would it be more efficient to simply clone from i to i-del? (Profiling did not show significant improvement.)
                    }
                }
            }
            if del > 0 {
                self.truncate(len - del);
            }
            // let mut i = 0;
            // self.retain(|stich| {
            //     if i < i_start {
            //         i += 1;
            //         true
            //     } else {
            //         fn_filter(stich)
            //     }
            // });
        }
        fn find_remove(&mut self, t: &T) -> bool where T: Eq {
            if let Some(i) = self.iter().position(|t_in_vec| t_in_vec==t) {
                self.swap_remove(i);
                true
            } else {
                false
            }
        }
    }
}}
impl_vecext!(Vec<T>);
impl_vecext!(ArrayVec<[T; 8]>);


trait TStichSize : TStaticValue<usize> {
    type Next: TStichSize;
}
macro_rules! define_stichsize{($stichsize:ident, $n:expr, $stichsize_next:ident) => {
    define_static_value!(pub $stichsize, usize, $n);
    impl TStichSize for $stichsize {
        type Next = $stichsize_next;
    }
}}
define_stichsize!(SStichSize0, 0, SStichSize1);
define_stichsize!(SStichSize1, 1, SStichSize2);
define_stichsize!(SStichSize2, 2, SStichSize3);
define_stichsize!(SStichSize3, 3, SStichSize4);
define_stichsize!(SStichSize4, 4, SStichSize0);
#[inline(always)]
fn find_relevant_stichs<
    StichSize: TStichSize,
    TFnSameParty: Fn(EPlayerIndex, EPlayerIndex)->bool,
>(
    stichseq: &mut SStichSequence,
    ahand: &EnumMap<EPlayerIndex, SHand>,
    rules: &dyn TRules,
    winidxcache: &SWinnerIndexCache,
    cluster: &SEquivalenceLists,
    epi_self: EPlayerIndex,
    fn_is_same_party: &TFnSameParty,
    vecstich_result: &mut Vec<SStich>,
) {
    if StichSize::VALUE==EPlayerIndex::SIZE {
        vecstich_result.push(unwrap!(stichseq.completed_stichs().last()).clone()); // must yield this one to callers
    } else {
        let epi_current = unwrap!(stichseq.current_stich().current_playerindex());
        macro_rules! dbg(($e:expr) => {$e});
        let mut veccard_allowed = dbg!(rules.all_allowed_cards(stichseq, &ahand[epi_current]));
        while let Some(card_allowed) = veccard_allowed.pop() {
            use crate::rules::card_points::*;
            let n_stich_before = vecstich_result.len();
            let mut ab_points_seen = [false; 12];
            let mut card_first = cluster
                .prevs(card_allowed)
                .take_while(|card| veccard_allowed.find_remove(card))
                .last()
                .unwrap_or(card_allowed);
            let ocard_last = cluster
                .nexts(card_allowed)
                .skip_while(|card| veccard_allowed.find_remove(card))
                .next();
            //println!("{} {:?}", card_first, ocard_last);
            let (mut card_lo, mut card_hi) = (card_first, card_first);
            loop {
                if assign_other(&mut ab_points_seen[points_card(card_first).as_num::<usize>()], true) {
                    if points_card(card_lo) > points_card(card_first) {
                        card_lo = card_first;
                    }
                    if points_card(card_hi) < points_card(card_first) {
                        card_hi = card_first;
                    }
                    stichseq.zugeben_and_restore_custom_winner_index(card_first, |stich| { debug_verify_eq!(winidxcache.get(stich), rules.winner_index(stich)) }, |stichseq| {
                        find_relevant_stichs::<StichSize::Next, _>(
                            stichseq,
                            ahand,
                            rules,
                            winidxcache,
                            cluster,
                            epi_self,
                            fn_is_same_party,
                            vecstich_result,
                        );
                    });
                }
                let ocard_next = cluster.next(card_first);
                if ocard_next==ocard_last {
                    break;
                } else if let Some(card_next) = verify!(ocard_next) {
                    card_first = card_next;
                }
            }
            if vecstich_result[n_stich_before..]
                .iter()
                .map(|stich|
                    // TODO is this correct? Do we have to rely on winner_index directly?
                    fn_is_same_party(epi_current, debug_verify_eq!(winidxcache.get(stich), rules.winner_index(stich)))
                )
                .all_equal()
            {
                { // the following represents a OthersMin player
                    if fn_is_same_party(epi_self, debug_verify_eq!(winidxcache.get(&vecstich_result[n_stich_before]), rules.winner_index(&vecstich_result[n_stich_before]))) {
                        vecstich_result.retain_from_to_end(n_stich_before, |stich| stich[epi_current]==card_lo);
                    } else {
                        vecstich_result.retain_from_to_end(n_stich_before, |stich| stich[epi_current]==card_hi)
                    }
                }

                /*{ // the following represents a MaxPerEpi player
                    if fn_is_same_party(epi_current, rules.winner_index(&vecstich_candidate[0])) {
                        vecstich_result.retain_from_to_end(n_stich_before, |stich| stich[epi_current]==card_hi);
                    } else {
                        vecstich_result.retain_from_to_end(n_stich_before, |stich| stich[epi_current]==card_lo)
                    }
                }*/
            }
        }
    }
}

pub fn suggest_card(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let hand_fixed = super::str_to_hand(&unwrap!(clapmatches.value_of("hand")))?;
    let veccard_as_played = &cardvector::parse_cards::<Vec<_>>(
        &unwrap!(clapmatches.value_of("cards_on_table")),
    ).ok_or_else(||format_err!("Could not parse played cards"))?;
    // TODO check that everything is ok (no duplicate cards, cards are allowed, current stich not full, etc.)
    let rules = crate::rules::parser::parse_rule_description_simple(&unwrap!(clapmatches.value_of("rules")))?;
    let rules = rules.as_ref();
    let stichseq = SStichSequence::new_from_cards(
        /*ekurzlang*/EKurzLang::checked_from_cards_per_player(
            /*n_stichs_complete*/veccard_as_played.len() / EPlayerIndex::SIZE
                + hand_fixed.cards().len()
        )
            .ok_or_else(|| format_err!("Cannot determine ekurzlang from {} and {:?}.", hand_fixed, veccard_as_played))?,
        veccard_as_played.iter().copied(),
        rules
    );
    let determinebestcard =  SDetermineBestCard::new(
        rules,
        &stichseq,
        &hand_fixed,
    );
    let epi_fixed = determinebestcard.epi_fixed;

    {
        let ahand = {
            let ahand_0 = unwrap!(all_possible_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules).next()).clone();
            EPlayerIndex::map_from_fn(|epi| {
                let mut veccard = ahand_0[epi].cards().clone();
                if_dbg_else!({veccard.shuffle(&mut rand::thread_rng())}{});
                SHand::new_from_vec(veccard)
            })
        };
        println!("{:?}", ahand);
        {
            fn internal_explore_2(
                stichseq: &mut SStichSequence,
                ahand: &mut EnumMap<EPlayerIndex, SHand>,
                setcard_played: SSetCard,
                rulestatecache: &mut SRuleStateCache,
                n_stichs_bound: usize,
                rules: &dyn TRules,
                winidxcache: &SWinnerIndexCache,
                map: &mut [HashSet::<SBeginning>; ERemainingCards::SIZE+1],
                fn_final: &mut impl FnMut(),
            ) {
                assert!(ahand.iter().map(|hand| hand.cards().len()).all_equal());
                let beginning = SBeginning::new(
                    setcard_played,
                    &stichseq,
                    debug_verify_eq!(
                        &rulestatecache.changing,
                        &SRuleStateCacheChanging::new(
                            &stichseq,
                            &ahand,
                            |stich| debug_verify_eq!(winidxcache.get(stich), rules.winner_index(stich)),
                        )
                    ),
                );
                if !map[stichseq.completed_stichs().len()].insert(beginning) {
                    return;
                } else if ahand[EPlayerIndex::EPI0].cards().len()>0 && stichseq.completed_stichs().len() < n_stichs_bound {
                    let mut vecstich = Vec::with_capacity(4096);
                    use crate::primitives::card::card_values::*;
                    find_relevant_stichs::<SStichSize0, _>(
                        stichseq,
                        ahand,
                        rules,
                        winidxcache,
                        &SEquivalenceLists::new(
                            &[
                                &[EO, GO, HO, SO, EU, GU, HU, SU, HA, HZ, HK, H9, H8, H7],
                                &[EA, EZ, EK, E9, E8, E7],
                                &[GA, GZ, GK, G9, G8, G7],
                                &[SA, SZ, SK, S9, S8, S7],
                            ],
                            stichseq,
                        ),
                        EPlayerIndex::EPI0,
                        &|epi_lhs, epi_rhs| (epi_lhs==EPlayerIndex::EPI0)==(epi_rhs==EPlayerIndex::EPI0),
                        &mut vecstich,
                    );
                    for stich in vecstich.iter() {
                        assert_eq!(stichseq.current_stich().size(), 0);
                        assert_eq!(stichseq.current_stich().first_playerindex(), stich.first_playerindex());
                        for (epi, &card) in stich.iter() {
                            ahand[epi].play_card_2(card);
                        }
                        let slccard = stich.elements_in_order();
                        assert_eq!(slccard.len(), EPlayerIndex::SIZE);
                        macro_rules! winner_index{() => { |stich| {
                            debug_verify_eq!(winidxcache.get(stich), rules.winner_index(stich))
                        }}};
                        // TODO introduce push_pop_stich
                        let mut setcard_played_new = setcard_played;
                        setcard_played_new.play(slccard[0]);
                        setcard_played_new.play(slccard[1]);
                        setcard_played_new.play(slccard[2]);
                        setcard_played_new.play(slccard[3]);
                        stichseq.zugeben_and_restore_custom_winner_index(slccard[0], winner_index!(), |stichseq| {
                            stichseq.zugeben_and_restore_custom_winner_index(slccard[1], winner_index!(), |stichseq| {
                                stichseq.zugeben_and_restore_custom_winner_index(slccard[2], winner_index!(), |stichseq| {
                                    stichseq.zugeben_and_restore_custom_winner_index(slccard[3], winner_index!(), |stichseq| {
                                        let unregisterstich = rulestatecache.register_stich(stich, winner_index!()(&stich));
                                        internal_explore_2(
                                            stichseq,
                                            ahand,
                                            setcard_played_new,
                                            rulestatecache,
                                            n_stichs_bound,
                                            rules,
                                            winidxcache,
                                            map,
                                            fn_final,
                                            );
                                        rulestatecache.unregister_stich(unregisterstich);
                                    });
                                });
                            });
                        });
                        for (epi, &card) in stich.iter() {
                            ahand[epi].add_card(card);
                        }
                    }
                } else {
                    fn_final();
                }
            }
            fn doit(
                stichseq: &mut SStichSequence,
                ahand: &mut EnumMap<EPlayerIndex, SHand>,
                rules: &dyn TRules,
                winidxcache: &SWinnerIndexCache,
                n_stichs_bound: usize,
            ) {
                let mut n_count = 0;
                internal_explore_2(
                    stichseq,
                    ahand,
                    {
                        let mut setcard = SSetCard::new();
                        for stich in stichseq.visible_stichs() {
                            for card in stich.elements_in_order() {
                                setcard.play(*card);
                            }
                        }
                        setcard
                    },
                    &mut SRuleStateCache::new(
                        stichseq,
                        ahand,
                        |stich| {
                            debug_verify_eq!(winidxcache.get(stich), rules.winner_index(stich))
                        },
                    ),
                    n_stichs_bound,
                    rules,
                    winidxcache,
                    &mut Default::default(),
                    /*fn_final*/&mut || {
                        n_count += 1;
                    },
                );
                println!("n_count={}", n_count);
            }
            doit(
                &mut SStichSequence::new(EKurzLang::Lang),
                &mut ahand.clone(),
                rules,
                &SWinnerIndexCache::new(&ahand, rules),
                unwrap!(unwrap!(clapmatches.value_of("depth")).parse()),
            );
        }
    }
    panic!("Nur bis hier.");


    let b_verbose = clapmatches.is_present("verbose");
    let eremainingcards = unwrap!(ERemainingCards::checked_from_usize(remaining_cards_per_hand(&stichseq)[epi_fixed] - 1));
    let determinebestcardresult = { // we are interested in payout => single-card-optimization useless
        macro_rules! forward{(($itahand: expr), ($func_filter_allowed_cards: expr), ($foreachsnapshot: ident),) => {{ // TODORUST generic closures
            determine_best_card(
                &determinebestcard,
                $itahand
                    .inspect(|ahand| {
                        if b_verbose { // TODO? dispatch statically
                            // TODO make output pretty
                            for hand in ahand.iter() {
                                print!("{} | ", hand);
                            }
                            println!("");
                        }
                    }),
                $func_filter_allowed_cards,
                &$foreachsnapshot::new(
                    rules,
                    epi_fixed,
                    /*tpln_stoss_doubling*/(0, 0), // TODO? make customizable
                    /*n_stock*/0, // TODO? make customizable
                ),
                /*opath_out_dir*/None, // TODO? make customizable
            )
        }}}
        enum VChooseItAhand {
            All,
            Sample(usize),
        };
        use VChooseItAhand::*;
        let oiteratehands = if_then_some!(let Some(str_itahand)=clapmatches.value_of("simulate_hands"),
            if "all"==str_itahand.to_lowercase() {
                All
            } else {
                Sample(str_itahand.parse()?)
            }
        );
        use ERemainingCards::*;
        let orelation = if_then_some!(let Some(str_constrain_hands)=clapmatches.value_of("constrain_hands"), {
            let relation = str_constrain_hands.parse::<VConstraint>().map_err(|()|format_err!("Cannot parse hand constraints"))?;
            if b_verbose {
                println!("Constraint parsed as: {}", relation);
            }
            relation
        });
        cartesian_match!(
            forward,
            match ((oiteratehands, eremainingcards)) {
                (Some(All), _)|(None, _1)|(None, _2)|(None, _3)|(None, _4) => (
                    all_possible_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)
                        .filter(|ahand| orelation.as_ref().map_or(true, |relation|
                            relation.eval(ahand, rules)
                        ))
                ),
                (Some(Sample(n_samples)), _) => (
                    forever_rand_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)
                        .filter(|ahand| orelation.as_ref().map_or(true, |relation|
                            relation.eval(ahand, rules)
                        ))
                        .take(n_samples)
                ),
                (None, _5)|(None, _6)|(None, _7)|(None, _8) => (
                    forever_rand_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)
                        .filter(|ahand| orelation.as_ref().map_or(true, |relation|
                            relation.eval(ahand, rules)
                        ))
                        .take(/*n_suggest_card_samples*/50)
                ),
            },
            match ((
                if_then_some!(let Some(str_tpln_branching) = clapmatches.value_of("branching"), {
                    let (str_lo, str_hi) = str_tpln_branching
                        .split(',')
                        .collect_tuple()
                        .ok_or_else(|| format_err!("Could not parse branching"))?;
                    let (n_lo, n_hi) = (str_lo.trim().parse::<usize>()?, str_hi.trim().parse::<usize>()?);
                    if_then_some!(n_lo < hand_fixed.cards().len(), {
                        if b_verbose {
                            println!("Branching bounds are large enough to eliminate branching factor.");
                        }
                        (n_lo, n_hi)
                    })
                }),
                eremainingcards
            )) {
                (Some(None), _)|(None,_1)|(None,_2)|(None,_3)|(None,_4) => (&|_,_| (/*no filtering*/)),
                (Some(Some((n_lo, n_hi))), _) => (&branching_factor(move |_stichseq| {
                    let n_lo = n_lo.max(1);
                    (n_lo, (n_hi.max(n_lo+1)))
                })),
                (None,_5)|(None,_6)|(None,_7)|(None,_8) => (&branching_factor(|_stichseq| (1, 3))),
            },
            match ((clapmatches.value_of("prune"), eremainingcards)) {
                (Some("none"),_)|(_, _1)|(_, _2)|(_, _3) => (SMinReachablePayout),
                (Some("hint"),_)|(_, _4)|(_, _5)|(_, _6)|(_, _7)|(_, _8) => (SMinReachablePayoutLowerBoundViaHint),
            },
        )
    };
    // TODO interface should probably output payout interval per card
    let mut veccardminmax = determinebestcardresult.cards_and_ts().collect::<Vec<_>>();
    veccardminmax.sort_unstable_by_key(|&(_card, minmax)| minmax);
    veccardminmax.reverse(); // descending
    // crude formatting: treat all numbers as f32, and convert structured input to a plain number table
    const N_COLUMNS : usize = EMinMaxStrategy::SIZE*3;
    let mut vecaf = Vec::new();
    let mut veclinestrings : Vec<(/*card*/String, /*numbers*/[String; N_COLUMNS])> = Vec::new();
    let mut an_width = [0; N_COLUMNS];
    let mut af_min = [f32::MAX; N_COLUMNS];
    let mut af_max = [f32::MIN; N_COLUMNS];
    for (card, minmax) in veccardminmax {
        let af = [
            minmax.0[EMinMaxStrategy::OthersMin].min().as_num::<f32>(),
            minmax.0[EMinMaxStrategy::OthersMin].avg(),
            minmax.0[EMinMaxStrategy::OthersMin].max().as_num::<f32>(),
            minmax.0[EMinMaxStrategy::MaxPerEpi].min().as_num::<f32>(),
            minmax.0[EMinMaxStrategy::MaxPerEpi].avg(),
            minmax.0[EMinMaxStrategy::MaxPerEpi].max().as_num::<f32>(),
        ];
        let astr = [
            format!("{} ", af[0]),
            format!("{:.2} ", af[1]),
            format!("{} ", af[2]),
            format!("{} ", af[3]),
            format!("{:.2} ", af[4]),
            format!("{}", af[5]),
        ];
        for (n_width, str) in an_width.iter_mut().zip(astr.iter()) {
            *n_width = (*n_width).max(str.len());
        }
        for (f_min, f_max, f) in izip!(af_min.iter_mut(), af_max.iter_mut(), af.iter()) {
            // TODO? assign_min/assign_max
            *f_min = f_min.min(*f);
            *f_max = f_max.max(*f);
        }
        veclinestrings.push((format!("{}", card), astr));
        vecaf.push(af);
    }
    for ((card, astr), af) in veclinestrings.iter().zip(vecaf) {
        print!("{}: ", card); // all cards have same width
        for (str_num, f, n_width, f_min, f_max) in izip!(astr.iter(), af.iter(), an_width.iter(), af_min.iter(), af_max.iter()) {
            use termcolor::*;
            let mut stdout = StandardStream::stdout(if atty::is(atty::Stream::Stdout) {
                ColorChoice::Auto
            } else {
                ColorChoice::Never
            });
            #[allow(clippy::float_cmp)]
            if f_min!=f_max {
                let mut set_color = |color| {
                    unwrap!(stdout.set_color(ColorSpec::new().set_fg(Some(color))));
                };
                if f==f_min {
                    set_color(Color::Red);
                } else if f==f_max {
                    set_color(Color::Green);
                }
            }
            print!("{:>width$}", str_num, width=n_width);
            unwrap!(stdout.reset());
        }
        println!();
    }
    Ok(())
}
