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
    ) -> R {
        match self {
            VConstraint::Not(constraint) => fn_bool(!constraint.eval(ahand, rules)),
            VConstraint::Num(numval) => fn_usize(numval.eval(ahand, rules)),
            VConstraint::Relation{numval_lhs, ord, numval_rhs} => fn_bool(*ord == numval_lhs.eval(ahand, rules).cmp(&numval_rhs.eval(ahand, rules))),
            VConstraint::Conjunction(constraint_lhs, constraint_rhs) => fn_bool(constraint_lhs.eval(ahand, rules) && constraint_rhs.eval(ahand, rules)),
            VConstraint::Disjunction(constraint_lhs, constraint_rhs) => fn_bool(constraint_lhs.eval(ahand, rules) || constraint_rhs.eval(ahand, rules)),
            VConstraint::Rhai(path) => {
                let mut engine = rhai::Engine::new();
                let mut scope = rhai::Scope::new();
                let ast = unwrap!(engine.compile_file(path.clone()));
                engine
                    .register_type::<EnumMap<EPlayerIndex, SHand>>()
                    .register_indexer_get(|enummap: &mut EnumMap<EPlayerIndex, SHand>, i: /*Rhai by default uses i64*/i64| -> SHand {
                        enummap[unwrap!(EPlayerIndex::checked_from_usize(i.as_num::<usize>()))].clone()
                    })
                    .register_type::<SHand>()
                    .register_fn("to_string", SHand::to_string)
                    .register_fn("contains", SHand::contains)
                    .register_fn("cards", |hand: /*TODO can we borrow here?*/SHand| -> Vec<SCard> {
                        hand.cards().to_vec()
                    })
                    .register_fn("count", |hand: SHand, rules: Box<&dyn TRules>, eschlag: ESchlag| {
                        hand.cards().iter().copied().filter(|card| card.schlag()==eschlag).count()
                    })
                    .register_type::<SCard>()
                    .register_fn("farbe", SCard::farbe)
                    .register_fn("schlag", SCard::schlag)
                    .register_fn("to_string", SCard::to_string)
                    .register_type::<&dyn TRules>()
                ;
                // TODO all this is ugly
                let resn : Result</*Rhai by default uses i64.*/i64,_> = engine.call_fn(&mut scope, &ast, "inspect", (ahand.clone(), rules.box_clone()));
                let resb : Result<bool,_> = engine.call_fn(&mut scope, &ast, "inspect", (ahand.clone(), rules.box_clone()));
                if let Ok(ref n) = resn {
                    fn_usize(n.as_num::<usize>())
                } else if let Ok(ref b) = resb {
                    fn_bool(*b)
                } else {
                    todo!("{:?}\n{:?}\nProbably replace all this by a fn_dynamic", resn, resb);
                }
            },
        }
    }
    pub fn eval(&self, ahand: &EnumMap<EPlayerIndex, SHand>, rules: &dyn TRules) -> bool {
        self.internal_eval(ahand, rules, |b| b, |n| n!=0)
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

