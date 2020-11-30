use crate::ai::{*, handiterators::*, suspicion::*};
use crate::game::SStichSequence;
use crate::primitives::*;
use crate::util::*;
use crate::rules::*;
use crate::cardvector::*;
use itertools::*;
use combine::{char::*, *};
use fxhash::FxHashMap as HashMap;

plain_enum_mod!(moderemainingcards, ERemainingCards {_1, _2, _3, _4, _5, _6, _7, _8,});

type SWinnerIndexCacheArr<T> = EnumMap<EPlayerIndex, [[[[T; 8]; 8]; 8]; 8]>; // TODO "8" should be unwrap!(EPlayerIndex::values().map(|ekurzlang| ekurzlang.cards_per_player()).max())

struct SWinnerIndexCache {
    mapcardi: EnumMap<SCard, usize>,
    aaaaepi: SWinnerIndexCacheArr<EPlayerIndex>,
    //if_dbg_else!({aaaab: SWinnerIndexCacheArr<bool>}{}),
}

impl SWinnerIndexCache {
    fn new(ahand: &EnumMap<EPlayerIndex, SHand>, rules: &dyn TRules) -> Self {
        let aaaaepi = EPlayerIndex::map_from_fn(|_| [[[[EPlayerIndex::EPI0; 8]; 8]; 8]; 8]);
        let mut mapcardi = SCard::map_from_fn(|_| 1000);
        //if_dbg_else!(let mut aaaab = Default::default());
        for epi in EPlayerIndex::values() {
            for (i_card, card) in ahand[epi].cards().iter().enumerate() {
                mapcardi[*card] = i_card;
            }
        }
        let mut slf = Self {
            aaaaepi,
            mapcardi,
            //if_dbg_else!({aaaab,}{})
        };
        for epi in EPlayerIndex::values() {
            for card_0 in ahand[epi.wrapping_add(0)].cards().iter().copied() {
                for card_1 in ahand[epi.wrapping_add(1)].cards().iter().copied() {
                    for card_2 in ahand[epi.wrapping_add(2)].cards().iter().copied() {
                        for card_3 in ahand[epi.wrapping_add(3)].cards().iter().copied() {
                            let stich = SStich::new_full(epi, [card_0, card_1, card_2, card_3]);
                            let slccard = stich.elements_in_order();
                            slf.aaaaepi[epi][slf.mapcardi[slccard[0]]][slf.mapcardi[slccard[1]]][slf.mapcardi[slccard[2]]][slf.mapcardi[slccard[3]]] = rules.winner_index(&stich);
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
        self.aaaaepi[stich.first_playerindex()][self.mapcardi[slccard[0]]][self.mapcardi[slccard[1]]][self.mapcardi[slccard[2]]][self.mapcardi[slccard[3]]]
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

pub fn suggest_card(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let b_verbose = clapmatches.is_present("verbose");
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
        let ahand = unwrap!(all_possible_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules).next()).clone();
        println!("{:?}", ahand);
        {
            #[derive(Clone, Debug, Eq, PartialEq, Hash)]
            struct SBeginning {
                setcard: EnumMap<SCard, bool>,
                pointstichcount_epi0: SPointStichCount,
                pointstichcount_other: SPointStichCount,
                epi_current: EPlayerIndex,
            };
            impl SBeginning {
                fn new(stichseq: &SStichSequence, rulestatecachechanging: &SRuleStateCacheChanging) -> Self {
                    let mut setcard = SCard::map_from_fn(|_| false);
                    for stich in stichseq.visible_stichs() {
                        for (_, card) in stich.iter() {
                            assert!(!setcard[*card]);
                            setcard[*card] = true;
                        }
                    }
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
            use arrayvec::ArrayVec;
            struct SCluster {
                veccard_trumpf: ArrayVec<[SCard; 14]>,
                aveccard_farbe: [ArrayVec<[SCard; 6]>; 3],
            }
            impl SCluster {
                fn new(rules: &dyn TRules, stichseq: &SStichSequence) -> Self {
                    use crate::primitives::card::card_values::*;
                    let mut slf = Self {
                        veccard_trumpf: ArrayVec::from([EO, GO, HO, SO, EU, GU, HU, SU, HA, HZ, HK, H9, H8, H7]),
                        aveccard_farbe: [
                            ArrayVec::from([EA, EZ, EK, E9, E8, E7]),
                            ArrayVec::from([GA, GZ, GK, G9, G8, G7]),
                            ArrayVec::from([SA, SZ, SK, S9, S8, S7]),
                        ],
                    };
                    // eliminate already played cards to close gaps
                    for stich in stichseq.completed_stichs().iter() {
                        for (_epi, card_played) in stich.iter() {
                            use VTrumpfOrFarbe::*;
                            use EFarbe::*;
                            match rules.trumpforfarbe(*card_played) {
                                Trumpf => slf.veccard_trumpf.retain(|card| card!=card_played),
                                Farbe(Eichel) => slf.aveccard_farbe[0].retain(|card| card!=card_played),
                                Farbe(Gras) => slf.aveccard_farbe[1].retain(|card| card!=card_played),
                                Farbe(Schelln) => slf.aveccard_farbe[2].retain(|card| card!=card_played),
                                Farbe(Herz) => panic!("TODO customize per rules"),
                            }
                        }
                    }
                    slf
                }
                fn get_equiv(&self, rules: &dyn TRules, card: SCard) -> &[SCard] {
                    use VTrumpfOrFarbe::*;
                    use EFarbe::*;
                    match rules.trumpforfarbe(card) {
                        Trumpf => &self.veccard_trumpf,
                        Farbe(Eichel) => &self.aveccard_farbe[0],
                        Farbe(Gras) => &self.aveccard_farbe[1],
                        Farbe(Schelln) => &self.aveccard_farbe[2],
                        Farbe(Herz) => panic!("TODO customize per rules"),
                    }
                }
            }
            fn internal_explore_2(
                stichseq: &SStichSequence,
                ahand: &EnumMap<EPlayerIndex, SHand>,
                rules: &dyn TRules,
                winidxcache: &SWinnerIndexCache,
                _n_stichseq_bound: usize,
                mut map: HashMap::<SBeginning, (SStichSequence, EnumMap<EPlayerIndex, SHand>)>,
            ) -> HashMap::<SBeginning, (SStichSequence, EnumMap<EPlayerIndex, SHand>)> {
                trait TStichSize : TStaticValue<usize> {
                    type Next: TStichSize;
                }
                macro_rules! define_stichsize{($stichsize:ident, $n:expr, $stichsize_next:ident) => {
                    define_static_value!(pub $stichsize, usize, $n);
                    impl TStichSize for $stichsize {
                        type Next = $stichsize_next;
                    }
                }};
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
                    cluster: &SCluster,
                    epi_self: EPlayerIndex,
                    fn_is_same_party: &TFnSameParty,
                    vecstich_result: &mut Vec<SStich>,
                ) {
                    if StichSize::VALUE==EPlayerIndex::SIZE {
                        vecstich_result.push(unwrap!(stichseq.completed_stichs().last()).clone()); // must yield this one to callers
                    } else {
                        let mut vecstich_relevant = Vec::new();
                        let epi_current = unwrap!(stichseq.current_stich().current_playerindex());
                        macro_rules! dbg(($e:expr) => {$e});
                        let mut veccard_allowed = dbg!(rules.all_allowed_cards(dbg!(stichseq), dbg!(&ahand[epi_current])));
                        while !veccard_allowed.is_empty() {
                            let veccard_equivalent = cluster.get_equiv(rules, veccard_allowed[0]);
                            {
                                for (_b_found, groupcard) in veccard_equivalent
                                    .iter()
                                    .group_by(|card| {
                                        if let Some(i) = veccard_allowed.iter().position(|card_allowed| card_allowed==*card) {
                                            veccard_allowed.swap_remove(i);
                                            true
                                        } else {
                                            false
                                        }
                                    })
                                    .into_iter()
                                    .filter(|(b_found, _)| *b_found)
                                {

                                    use crate::rules::card_points::*;
                                    let mut vecstich_candidate = Vec::new();
                                    let (mut ocard_lo, mut ocard_hi) = (None, None);
                                    let mut ab_points_seen = [false; 12];
                                    for &card in groupcard.filter(|card| {
                                        let b_seen : &mut bool = &mut ab_points_seen[points_card(**card).as_num::<usize>()];
                                        if *b_seen {
                                            return false;
                                        } else {
                                            *b_seen = true;
                                            return true;
                                        }
                                    }) {
                                        if ocard_lo.is_none() || points_card(unwrap!(ocard_lo)) > points_card(card) {
                                            ocard_lo = Some(card);
                                        }
                                        if ocard_hi.is_none() || points_card(unwrap!(ocard_hi)) < points_card(card) {
                                            ocard_hi = Some(card);
                                        }
                                        stichseq.zugeben_and_restore_custom_winner_index(card, |stich| { debug_verify_eq!(winidxcache.get(stich), rules.winner_index(stich)) }, |stichseq| {
                                            find_relevant_stichs::<StichSize::Next, _>(
                                                stichseq,
                                                ahand,
                                                rules,
                                                winidxcache,
                                                cluster,
                                                epi_self,
                                                fn_is_same_party,
                                                &mut vecstich_candidate,
                                            );
                                        });
                                    }
                                    dbg!(&vecstich_candidate);
                                    assert!(!vecstich_candidate.is_empty());
                                    if dbg!(vecstich_candidate
                                        .iter()
                                        .map(|stich|
                                            // TODO is this correct? Do we have to rely on winner_index directly?
                                            fn_is_same_party(epi_current, debug_verify_eq!(winidxcache.get(stich), rules.winner_index(stich)))
                                        )
                                        .all_equal()
                                    ) {
                                        //println!("Merging: {:?}", &vecstich_candidate);
                                        fn extend_with(
                                            vecstich: &mut Vec<SStich>,
                                            itstich: impl IntoIterator<Item=SStich>,
                                            fn_filter: impl Fn(&SStich)->bool,
                                        ) {
                                            vecstich.extend(itstich
                                                .into_iter()
                                                .filter(fn_filter)
                                                //.inspect(|stich| println!("{}", stich))
                                            );
                                        }

                                        { // the following represents a OthersMin player
                                            if fn_is_same_party(epi_self, debug_verify_eq!(winidxcache.get(&vecstich_candidate[0]), rules.winner_index(&vecstich_candidate[0]))) {
                                                extend_with(&mut vecstich_relevant, vecstich_candidate, |stich| stich[epi_current]==unwrap!(ocard_lo));
                                            } else {
                                                extend_with(&mut vecstich_relevant, vecstich_candidate, |stich| stich[epi_current]==unwrap!(ocard_hi))
                                            }
                                        }

                                        /*{ // the following represents a MaxPerEpi player
                                            if fn_is_same_party(epi_current, rules.winner_index(&vecstich_candidate[0])) {
                                                extend_with(&mut vecstich_relevant, vecstich_candidate, |stich| stich[epi_current]==unwrap!(ocard_hi));
                                            } else {
                                                extend_with(&mut vecstich_relevant, vecstich_candidate, |stich| stich[epi_current]==unwrap!(ocard_lo))
                                            }
                                        }*/
                                    } else {
                                        vecstich_relevant.extend(vecstich_candidate);
                                    }
                                }
                            }
                        }

                        vecstich_result.extend(vecstich_relevant);
                    }
                }
                let mut vecstich = Vec::new();
                find_relevant_stichs::<SStichSize0, _>(
                    &mut stichseq.clone(),
                    ahand,
                    rules,
                    winidxcache,
                    &SCluster::new(rules, stichseq),
                    EPlayerIndex::EPI0,
                    &|epi_lhs, epi_rhs| (epi_lhs==EPlayerIndex::EPI0)==(epi_rhs==EPlayerIndex::EPI0),
                    &mut vecstich,
                );


                let n_map_len_before = map.len();
                for stich in vecstich.iter() {
                    let mut stichseq = stichseq.clone();
                    assert_eq!(stichseq.current_stich().size(), 0);
                    assert_eq!(stichseq.current_stich().first_playerindex(), stich.first_playerindex());
                    let mut ahand = ahand.clone();
                    for (epi, &card) in stich.iter() {
                        stichseq.zugeben_custom_winner_index(card, |stich| {
                            debug_verify_eq!(winidxcache.get(stich), rules.winner_index(stich))
                        });
                        ahand[epi].play_card(card);
                    }
                    map.insert(
                        SBeginning::new(
                            &stichseq,
                            &SRuleStateCacheChanging::new(
                                &stichseq,
                                &ahand,
                                |stich| debug_verify_eq!(winidxcache.get(stich), rules.winner_index(stich)),
                            ),
                        ),
                        (stichseq, ahand)
                    );
                }
                let n_map_len_intermediate = map.len();
                // map.retain(|_, (stichseq, ahand)| {
                //     let payouthints = rules.payouthints(
                //         &stichseq,
                //         &ahand,
                //         &SRuleStateCache::new(
                //             &stichseq,
                //             &ahand,
                //             |stich| rules.winner_index(stich),
                //         ),
                //     );
                //     match (payouthints[EPlayerIndex::EPI0].lower_bound(), payouthints[EPlayerIndex::EPI0].upper_bound()) {
                //         (Some(payout_lo), _) if 0 <= payout_lo.payout_including_stock(0, (0, 0)) => false,
                //         (_, Some(payout_hi)) if payout_hi.payout_including_stock(0, (0, 0))<= 0 => false,
                //         _ => true,
                //     }
                // });
                println!("{} states compressed to {} to {} ({} total)", vecstich.len(), n_map_len_intermediate - n_map_len_before, map.len() - n_map_len_before, map.len());
                map



            }
            #[derive(new)]
            struct SStep {
                i_stichseq_depth: usize,
                n_batch: usize,
            }
            fn doit(
                ittplstichseqahand: impl Iterator<Item=(SStichSequence, EnumMap<EPlayerIndex, SHand>)>,
                rules: &dyn TRules,
                winidxcache: &SWinnerIndexCache,
                slcstep: &[SStep],
                f_percent_lo: f32,
                f_percent_hi: f32,
            ) {
                println!("{:03}% at depth: {}", f_percent_lo, slcstep.len());
                if let Some((step, slcstep_rest)) = slcstep.split_first() {
                    let mut map = Default::default();
                    for (stichseq, ahand) in ittplstichseqahand {
                        map = internal_explore_2(
                            &stichseq,
                            &ahand,
                            rules,
                            winidxcache,
                            step.i_stichseq_depth,
                            map,
                        );
                    }
                    let f_chunks = (map.len() / step.n_batch + 1) as f32;
                    let percentage = |i_chunk| (i_chunk as f32/f_chunks)*(f_percent_hi-f_percent_lo)+f_percent_lo;
                    for (i_chunk, chunk) in map.into_iter().chunks(step.n_batch).into_iter().enumerate() {
                        doit(
                            Box::new(chunk.map(|(_, (stichseq, ahand))| (stichseq, ahand))) as Box<dyn Iterator<Item=(SStichSequence, EnumMap<EPlayerIndex, SHand>)>>,
                            rules,
                            winidxcache,
                            slcstep_rest,
                            percentage(i_chunk),
                            percentage(i_chunk + 1),
                        );
                    }
                } else {
                    // for (mut stichseq, mut ahand) in ittplstichseqahand {
                    //     explore_snapshots(
                    //         &mut ahand,
                    //         rules,
                    //         &mut stichseq,
                    //         &|_, _| (),
                    //         &SMinReachablePayoutLowerBoundViaHint::new(
                    //             rules,
                    //             EPlayerIndex::EPI0,
                    //             (0, 0),
                    //             0,
                    //         ),
                    //         None,
                    //     );
                    // }
                }
            }
            let vecstep : Vec<_> = unwrap!(clapmatches.value_of("batch")).split(',').map(|str_step| {
                let (str_depth, str_chunk) = unwrap!(str_step.split(' ').collect_tuple());
                SStep::new(unwrap!(str_depth.parse()), unwrap!(str_chunk.parse()))
            }).collect();
            doit(
                std::iter::once((SStichSequence::new(EKurzLang::Lang), ahand.clone())),
                rules,
                &SWinnerIndexCache::new(&ahand, rules),
                &vecstep,
                0.,
                100.,
            );
        }
    }
    panic!("Nur bis hier.");


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
