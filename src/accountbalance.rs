// this stores the how much money each player currently has

use stich::*;

pub struct SAccountBalance {
    m_an : [isize; 4],
}

impl SAccountBalance {
    pub fn new() -> SAccountBalance {
        SAccountBalance { m_an : [0, 0, 0, 0] }
    }

    fn assert_invariant(&self) {
        // TODO Rust: Can we use iter().sum?
        assert_eq!(self.m_an.iter().fold(0, |n_acc, n| n_acc + n), 0);
    }

    pub fn apply_payout(&mut self, an_payout: &[isize; 4]) {
        self.assert_invariant();
        for eplayerindex in 0..4 {
            self.m_an[eplayerindex] = self.m_an[eplayerindex] + an_payout[eplayerindex];
        }
        self.assert_invariant();
    }

    pub fn get(&self, eplayerindex: EPlayerIndex) -> isize {
        self.assert_invariant();
        self.m_an[eplayerindex]
    }
}

