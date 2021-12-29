use crate::primitives::*;
use crate::util::{*, parser::*};
use crate::rules::*;
use crate::cardvector::*;
use combine::{char::*, *};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VNumVal {
    Const(usize),
    Card(SCard, EPlayerIndex),
    TrumpfOrFarbe(VTrumpfOrFarbe, EPlayerIndex),
    Schlag(ESchlag, EPlayerIndex),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VConstraint {
    Not(Box<VConstraint>),
    Num(VNumVal),
    Relation {
        numval_lhs: VNumVal,
        ord: std::cmp::Ordering,
        numval_rhs: VNumVal,
    },
    Conjunction(Box<VConstraint>, Box<VConstraint>),
    Disjunction(Box<VConstraint>, Box<VConstraint>),
    Rhai(std::path::PathBuf),
}

impl VNumVal {
    pub fn eval(&self, ahand: &EnumMap<EPlayerIndex, SHand>, rules: &dyn TRules) -> usize {
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
    pub fn internal_eval<R>(
        &self,
        ahand: &EnumMap<EPlayerIndex, SHand>,
        rules: &dyn TRules,
        fn_bool: impl Fn(bool)->R,
        fn_usize: impl Fn(usize)->R,
        fn_rhai: impl Fn(Option<rhai::Dynamic>)->R,
    ) -> R {
        match self {
            VConstraint::Not(constraint) => fn_bool(!constraint.eval(ahand, rules)),
            VConstraint::Num(numval) => fn_usize(numval.eval(ahand, rules)),
            VConstraint::Relation{numval_lhs, ord, numval_rhs} => fn_bool(*ord == numval_lhs.eval(ahand, rules).cmp(&numval_rhs.eval(ahand, rules))),
            VConstraint::Conjunction(constraint_lhs, constraint_rhs) => fn_bool(constraint_lhs.eval(ahand, rules) && constraint_rhs.eval(ahand, rules)),
            VConstraint::Disjunction(constraint_lhs, constraint_rhs) => fn_bool(constraint_lhs.eval(ahand, rules) || constraint_rhs.eval(ahand, rules)),
            VConstraint::Rhai(path) => {
                #[derive(Clone)]
                struct SRhaiParams {
                    ahand: EnumMap<EPlayerIndex, SHand>,
                    rules: Box<dyn TRules>,
                }
                type SRhaiUsize = i64; // TODO good idea?
                fn epi_from_rhai(i: SRhaiUsize) -> EPlayerIndex {
                    unwrap!(EPlayerIndex::checked_from_usize(i.as_num::<usize>()))
                }
                impl SRhaiParams {
                    fn count(&self, i_epi: SRhaiUsize, fn_pred: impl Fn(&SCard)->bool) -> SRhaiUsize {
                        self.ahand[epi_from_rhai(i_epi)]
                            .cards()
                            .iter()
                            .copied()
                            .filter(fn_pred)
                            .count()
                            .as_num::<SRhaiUsize>()
                    }
                }
                let mut engine = rhai::Engine::new();
                let mut scope = rhai::Scope::new();
                let ast = unwrap!(engine.compile_file(path.clone()));
                engine
                    .register_type::<SRhaiParams>()
                    .register_get("rules", |rhaiparams: &mut SRhaiParams| rhaiparams.rules.box_clone())
                    .register_get("ahand", |rhaiparams: &mut SRhaiParams| rhaiparams.ahand.clone())
                    .register_fn("card", |rhaiparams: SRhaiParams, i_epi: SRhaiUsize, card: SCard| {
                        rhaiparams.count(i_epi, |&card_hand| card_hand==card)
                    });
                for (str_trumpforfarbe, trumpforfarbe) in [
                    ("trumpf", VTrumpfOrFarbe::Trumpf),
                    ("eichel", VTrumpfOrFarbe::Farbe(EFarbe::Eichel)),
                    ("gras", VTrumpfOrFarbe::Farbe(EFarbe::Gras)),
                    ("herz", VTrumpfOrFarbe::Farbe(EFarbe::Herz)),
                    ("schelln", VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
                ] {
                    engine.register_fn(str_trumpforfarbe, move |rhaiparams: SRhaiParams, i_epi: SRhaiUsize| {
                        rhaiparams.count(i_epi, |&card| rhaiparams.rules.trumpforfarbe(card)==trumpforfarbe)
                    });
                }
                    // TODO schlag
                engine
                    .register_type::<EPlayerIndex>()
                    .register_fn("to_string", EPlayerIndex::to_string)
                    .register_type::<EnumMap<EPlayerIndex, SHand>>()
                    .register_indexer_get(|enummap: &mut EnumMap<EPlayerIndex, SHand>, i_epi: SRhaiUsize| -> SHand {
                        enummap[epi_from_rhai(i_epi)].clone()
                    })
                    .register_type::<SHand>()
                    .register_fn("to_string", |hand: SHand| hand.to_string())
                    .register_fn("contains", SHand::contains)
                    .register_fn("cards", |hand: /*TODO can we borrow here?*/SHand| -> Vec<SCard> {
                        hand.cards().to_vec()
                    })
                    .register_type::<SCard>()
                    .register_fn("farbe", SCard::farbe)
                    .register_fn("schlag", SCard::schlag)
                    .register_fn("to_string", SCard::to_string)
                    .register_type::<&dyn TRules>()
                    .register_fn("to_string", |rules: Box<dyn TRules>| rules.to_string())
                    .register_fn("playerindex", |rules: Box<dyn TRules>|
                        rules.playerindex().unwrap().to_usize().as_num::<SRhaiUsize>()
                    )
                ;
                let resdynamic : Result<rhai::Dynamic,_> = engine.call_fn(&mut scope, &ast, "inspect", (
                    SRhaiParams {
                        ahand: ahand.clone(),
                        rules: rules.box_clone(),
                    },
                ));
                match resdynamic {
                    Ok(dynamic) => {
                        if let Ok(n) = dynamic.as_int() {
                            fn_usize(n.as_num::<usize>())
                        } else if let Ok(b) = dynamic.as_bool() {
                            fn_bool(b)
                        } else {
                            println!("Unknown result data type.");
                            fn_rhai(Some(dynamic))
                        }
                    },
                    Err(e) => {
                        println!("Error evaluating script ({:?}).", e);
                        fn_rhai(None)
                    }
                }
            },
        }
    }
    pub fn eval(&self, ahand: &EnumMap<EPlayerIndex, SHand>, rules: &dyn TRules) -> bool {
        self.internal_eval(ahand, rules, |b| b, |n| n!=0, |_odynamic| false)
    }
}

impl std::fmt::Display for VConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            VConstraint::Not(constraint) => write!(f, "!({})", constraint),
            VConstraint::Num(numval) => write!(f, "{}", numval),
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
            VConstraint::Rhai(path) => write!(f, /*TODO proper formatting*/"Rhai({:?})", path),
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
            if let Some((ord, numval_rhs)) = otplordnumval_rhs {
                VConstraint::Relation{numval_lhs, ord, numval_rhs}
            } else {
                VConstraint::Num(numval_lhs)
            }
        })
    )
}
parser!(
    fn single_constraint_parser[I]()(I) -> VConstraint
        where [I: Stream<Item = char>]
    {
        single_constraint_parser_()
    }
);

fn constraint_parser_<I: Stream<Item=char>>() -> impl Parser<Input = I, Output = VConstraint>
    where I::Error: ParseError<I::Item, I::Range, I::Position>, // Necessary due to rust-lang/rust#24159
{
    macro_rules! make_bin_op_parser{($parser:ident, $chr:expr, $op:ident) => {
        let $parser = attempt((single_constraint_parser(), many1::<Vec<_>, _>((spaces(), char($chr), spaces()).with(single_constraint_parser()))))
            .map(|(constraint, vecconstraint)| unwrap!(std::iter::once(constraint).chain(vecconstraint.into_iter()).reduce(|constraint_lhs, constraint_rhs|
                VConstraint::$op(Box::new(constraint_lhs), Box::new(constraint_rhs))
            )));
    }}
    make_bin_op_parser!(conjunction, '&', Conjunction);
    make_bin_op_parser!(disjunction, '|', Disjunction);
    choice((
        conjunction,
        disjunction,
        attempt(single_constraint_parser()),
        attempt(
            (
                char('{'),
                many1(alpha_num().or(char('.')).or(char('/'))),
                char('}'),
            ).map(|(_chr_open_parenthesis, str_path, _chr_close_parenthesis): (_, String, _)| -> VConstraint {
                VConstraint::Rhai(unwrap!(str_path.parse()))
            }),
        ),
    ))
}

parser!(
    fn constraint_parser[I]()(I) -> VConstraint
        where [I: Stream<Item = char>]
    {
        constraint_parser_()
    }
);

#[test]
fn test_constraint_parser() {
    fn test_internal(str_in: &str, constraint: VConstraint) {
        assert_eq!(unwrap!(str_in.parse::<VConstraint>()), constraint);
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
    fn test_simple_greater_0(str_in: &str, numval: VNumVal) {
        let relation = Num(numval);
        test_internal(str_in, relation.clone());
        test_internal(&format!("!{}", str_in), Not(Box::new(relation.clone())));
        test_internal(&format!("!!{}", str_in), Not(Box::new(Not(Box::new(relation)))));
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
    test_internal(
        "e(1)&e(2)",
        Conjunction(
            Box::new(Num(TrumpfOrFarbe(Farbe(Eichel), EPI1))),
            Box::new(Num(TrumpfOrFarbe(Farbe(Eichel), EPI2))),
        )
    );
    test_internal(
        "e(1)|e(2)",
        Disjunction(
            Box::new(Num(TrumpfOrFarbe(Farbe(Eichel), EPI1))),
            Box::new(Num(TrumpfOrFarbe(Farbe(Eichel), EPI2))),
        )
    );
    // TODO more tests
}

impl std::str::FromStr for VConstraint {
    type Err = Error;
    fn from_str(str_in: &str) -> Result<Self, Self::Err> {
        parse_trimmed(str_in, "constraint", constraint_parser())
    }
}

