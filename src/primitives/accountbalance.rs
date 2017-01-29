// this stores the how much money each player currently has

use primitives::eplayerindex::*;
use util::*;

pub struct SAccountBalance {
    m_an : SEnumMap<EPlayerIndex, isize>,
    m_n_stock : isize,
}

impl SAccountBalance {
    pub fn new(an: SEnumMap<EPlayerIndex, isize>, n_stock: isize) -> SAccountBalance {
        let accountbalance = SAccountBalance {
            m_an : an,
            m_n_stock : n_stock,
        };
        accountbalance.assert_invariant();
        accountbalance
    }

    fn assert_invariant(&self) {
        assert_eq!(self.m_n_stock + self.m_an.iter().sum::<isize>(), 0);
    }

    pub fn apply_payout(&mut self, accountbalance: &SAccountBalance) {
        accountbalance.assert_invariant();
        self.assert_invariant();
        for epi in EPlayerIndex::values() {
            self.m_an[epi] += accountbalance.get_player(epi);
        }
        self.m_n_stock += accountbalance.get_stock();
        self.assert_invariant();
    }

    pub fn get_player(&self, epi : EPlayerIndex) -> isize {
        self.assert_invariant();
        self.m_an[epi]
    }

    pub fn get_stock(&self) -> isize {
        self.assert_invariant();
        self.m_n_stock
    }
}

