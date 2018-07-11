use std::{
    fmt,
    ops::Index,
    iter::FromIterator,
};
use util::*;

plain_enum_mod!{modefarbe, derive(Serialize, Deserialize,), map_derive(Serialize,), EFarbe {
    Eichel,
    Gras,
    Herz,
    Schelln,
}}

define_static_value!(pub SStaticFarbeEichel, EFarbe, EFarbe::Eichel);
define_static_value!(pub SStaticFarbeGras, EFarbe, EFarbe::Gras);
define_static_value!(pub SStaticFarbeHerz, EFarbe, EFarbe::Herz);
define_static_value!(pub SStaticFarbeSchelln, EFarbe, EFarbe::Schelln);

impl fmt::Display for EFarbe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            EFarbe::Eichel => "Eichel",
            EFarbe::Gras => "Gras",
            EFarbe::Herz => "Herz",
            EFarbe::Schelln => "Schelln",
        } )
    }
}

plain_enum_mod!{modeschlag, derive(Serialize, Deserialize,), map_derive(Serialize,), ESchlag {
    Ass,
    Zehn,
    Koenig,
    Ober,
    Unter,
    S9,
    S8,
    S7,
}}

define_static_value!(pub SStaticSchlagOber, ESchlag, ESchlag::Ober);
define_static_value!(pub SStaticSchlagUnter, ESchlag, ESchlag::Unter);

impl fmt::Display for ESchlag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

plain_enum_mod!{modekurzlang, EKurzLang {
    Kurz,
    Lang,
}}

impl EKurzLang {
    pub fn cards_per_player(self) -> usize {
        match self {
            EKurzLang::Kurz => 6,
            EKurzLang::Lang => 8,
        }
    }

    pub fn from_cards_per_player(n_cards_per_player: usize) -> EKurzLang {
        match n_cards_per_player {
            6 => EKurzLang::Kurz,
            8 => EKurzLang::Lang,
            _ => panic!("Cannot convert {} to EKurzLang.", n_cards_per_player),
        }
    }

    pub fn supports_card(self, card: SCard) -> bool {
        match self {
            EKurzLang::Lang => true,
            EKurzLang::Kurz => card.schlag()!=ESchlag::S7 && card.schlag()!=ESchlag::S8,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug, Serialize, Deserialize)]
pub struct SCard {
    n_internalrepresentation : u8,
}

impl fmt::Display for SCard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", 
            match self.farbe() {
                EFarbe::Eichel => "E",
                EFarbe::Gras => "G",
                EFarbe::Herz => "H",
                EFarbe::Schelln => "S",
            },
            match self.schlag() {
                ESchlag::S7 => "7",
                ESchlag::S8 => "8",
                ESchlag::S9 => "9",
                ESchlag::Zehn => "Z",
                ESchlag::Unter => "U",
                ESchlag::Ober => "O",
                ESchlag::Koenig => "K",
                ESchlag::Ass => "A",
            }
        )
    }
}

impl SCard {
    pub fn new(efarbe : EFarbe, eschlag : ESchlag) -> SCard {
        SCard{n_internalrepresentation : (efarbe.to_usize() * ESchlag::SIZE + eschlag.to_usize()).as_num()}
    }
    pub fn farbe(self) -> EFarbe {
        EFarbe::from_usize(self.n_internalrepresentation.as_num::<usize>() / ESchlag::SIZE)
    }
    pub fn schlag(self) -> ESchlag {
        ESchlag::from_usize(self.n_internalrepresentation.as_num::<usize>() % ESchlag::SIZE)
    }
    pub fn values(ekurzlang: EKurzLang) -> impl Iterator<Item=SCard> {
        iproduct!(
            EFarbe::values(),
            ESchlag::values()
        )
        .filter_map(move |(efarbe, eschlag)| {
            match ekurzlang { // prefer matching on custom enums over simple if/else
                EKurzLang::Kurz => if ESchlag::S7==eschlag || ESchlag::S8==eschlag {
                    return None;
                },
                EKurzLang::Lang => (),
            }
            Some(SCard::new(efarbe, eschlag))
        })
    }
}

#[test]
fn test_farbe_schlag_enumerators() {
    assert_eq!(EFarbe::values().count(), 4);
    assert_eq!(ESchlag::values().count(), 8);
}

#[test]
fn test_card_ctor() {
    for efarbe in EFarbe::values() {
        for eschlag in ESchlag::values() {
            assert_eq!(SCard::new(efarbe, eschlag).farbe(), efarbe);
            assert_eq!(SCard::new(efarbe, eschlag).schlag(), eschlag);
        }
    }
}

pub struct SCardMap<T> {
    aot : [Option<T>; 32],
}

impl<T> FromIterator<(SCard, T)> for SCardMap<T> {
    fn from_iter<ItPairCardT: IntoIterator<Item=(SCard, T)>>(itpaircardt : ItPairCardT) -> SCardMap<T> {
        SCardMap {
            aot : {
                let mut aot : [_; 32] = Default::default();
                for (card, t) in itpaircardt {
                    aot[card.n_internalrepresentation.as_num::<usize>()] = Some(t);
                }
                aot
            }
        }
    }
}

impl <T> Index<SCard> for SCardMap<T> {
    type Output = T;

    fn index(&self, card: SCard) -> &T {
        verify!(self.aot[card.n_internalrepresentation.as_num::<usize>()].as_ref()).unwrap()
    }
}
