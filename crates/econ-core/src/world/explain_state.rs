//! The explanations' monthly details on the `World` (feature `explain`; spec
//! `game/explanations`): what a tree needs beyond the registry's history and
//! the mechanics' month records, kept at each close. Outputs only, outside
//! the state hash.

use econ_types::Bani;

use super::World;

/// One close's details.
#[derive(Debug, Clone, Default)]
pub(crate) struct XpFrame {
    /// GDP by expenditure: consumption, government purchases, investment,
    /// exports, inventories, imports (bani, national).
    pub gdp: [i64; 6],
    /// Each good's term of the CPI: basket share × consumer price.
    pub terms: Vec<i128>,
    /// Employed and unemployed persons by region, weighted (as the close
    /// counts them).
    pub employed: Vec<i64>,
    pub unemployed: Vec<i64>,
    /// Each industry's wages paid over the month and its staff at the close.
    pub wages: Vec<i64>,
    pub staff: Vec<i64>,
    /// Whether the policy rate at the close is the player's (an order took
    /// effect; the advisory rule no longer sets it).
    pub rate_by_orders: bool,
}

/// The details kept.
#[derive(Debug, Clone, Default)]
pub(crate) struct XpState {
    /// Day 0's CPI terms.
    pub base_terms: Vec<i128>,
    /// One frame per close.
    pub frames: Vec<XpFrame>,
    /// The month being closed: GDP's parts and the industries' wages.
    gdp: [i64; 6],
    wages: Vec<i64>,
}

impl World {
    pub(super) fn xp_start(&mut self) {
        self.xp.base_terms = self.cpi_terms();
    }

    pub(super) fn xp_note_gdp(&mut self, parts: [i128; 6]) {
        self.xp.gdp = parts.map(|x| i64::try_from(x).unwrap_or(i64::MAX));
    }

    pub(super) fn xp_note_wages(&mut self, j: usize, x: Bani) {
        if self.xp.wages.len() <= j {
            self.xp.wages.resize(j + 1, 0);
        }
        self.xp.wages[j] = x.get();
    }

    /// The close's frame, at the end of its day.
    pub(super) fn xp_capture(&mut self) {
        let n_r = usize::from(self.pp.region.iter().copied().max().unwrap_or(0)) + 1;
        let (mut employed, mut unemployed) = (vec![0i64; n_r], vec![0i64; n_r]);
        for p in 0..self.persons.employer.len() {
            let w = i64::from(self.households.hh_weight[self.persons.household_id[p] as usize]);
            let r = usize::from(self.pp.region[p]);
            if self.pp.seeking[p] {
                unemployed[r] += w;
            } else if self.pp.status[p] != 0 {
                employed[r] += w;
            }
        }
        let n_i = self.industries.id.len();
        let mut staff = vec![0i64; n_i];
        for (u, &s) in self.units.staff.iter().enumerate() {
            staff[usize::from(self.units.industry[u])] += i64::try_from(s).unwrap_or(0);
        }
        let mut wages = std::mem::take(&mut self.xp.wages);
        wages.resize(n_i, 0);
        let frame = XpFrame {
            gdp: self.xp.gdp,
            terms: self.cpi_terms(),
            employed,
            unemployed,
            wages,
            staff,
            #[cfg(feature = "monetary")]
            rate_by_orders: self.cb_overridden(),
            #[cfg(not(feature = "monetary"))]
            rate_by_orders: false,
        };
        self.xp.frames.push(frame);
    }

    pub(crate) fn xp_frames(&self) -> &[XpFrame] {
        &self.xp.frames
    }

    pub(crate) fn xp_base_terms(&self) -> &[i128] {
        &self.xp.base_terms
    }
}
