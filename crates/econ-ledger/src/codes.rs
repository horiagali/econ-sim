//! Flow codes: the "why" of every posting (ADR-0007).
//!
//! `u16` values in ranges per module. **Append-only: never renumber or
//! reuse a code** — saves and golden runs depend on them. Eventually
//! generated from the glossary (ADR-0012); hand-written for the spikes.

/// The reason a posting happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FlowCode(pub u16);

impl FlowCode {
    // 1xxx — final demand
    /// Household consumption purchases.
    pub const CONSUMPTION: FlowCode = FlowCode(1001);
    /// Government purchases of goods and services.
    pub const GOV_PURCHASES: FlowCode = FlowCode(1002);
    /// Exports: the rest of the world pays for goods (M2: in deposits).
    pub const EXPORTS: FlowCode = FlowCode(1004);
    /// Imports: residents pay the rest of the world for goods, inputs
    /// included (M2: in deposits, at the starting price).
    pub const IMPORTS: FlowCode = FlowCode(1005);
    // 2xxx — income
    /// Wages paid by firms.
    pub const WAGES: FlowCode = FlowCode(2001);
    /// Interest on government debt (D4), to its holders.
    pub const INTEREST_GOV_DEBT: FlowCode = FlowCode(2002);
    /// Self-employment and informal income paid out of firm units' surplus
    /// to households (M2 placeholder of spec `economy/households`).
    pub const MIXED_INCOME: FlowCode = FlowCode(2003);
    // 3xxx — taxes
    /// Personal income tax.
    pub const TAX_INCOME: FlowCode = FlowCode(3001);
    /// Value-added tax collected on household purchases.
    pub const TAX_VAT: FlowCode = FlowCode(3002);
    /// Employers' labour insurance contribution (CAM).
    pub const TAX_EMPLOYER_CONTRIBUTION: FlowCode = FlowCode(3003);
    /// Employees' pension contribution (CAS) to the government.
    pub const CONTRIB_PENSION: FlowCode = FlowCode(3004);
    /// The part of CAS that goes to the person's Pillar II pension fund.
    pub const CONTRIB_PENSION_PILLAR2: FlowCode = FlowCode(3005);
    /// Employees' health contribution (CASS).
    pub const CONTRIB_HEALTH: FlowCode = FlowCode(3006);
    /// Corporate profit tax.
    pub const TAX_PROFIT: FlowCode = FlowCode(3007);
    /// Micro-enterprise tax on turnover.
    pub const TAX_MICRO: FlowCode = FlowCode(3008);
    // 4xxx — social transfers (numbering of spec `economy/budget-2021`)
    /// Old-age and survivors' pensions.
    pub const BENEFIT_PENSION: FlowCode = FlowCode(4001);
    /// Child allowance.
    pub const BENEFIT_FAMILY: FlowCode = FlowCode(4003);
    /// Unemployment benefit.
    pub const BENEFIT_UNEMPLOYMENT: FlowCode = FlowCode(4004);
    /// Guaranteed minimum income.
    pub const BENEFIT_SOCIAL_ASSISTANCE: FlowCode = FlowCode(4006);
    /// Other social protection (and one-off transfers).
    pub const BENEFIT_OTHER: FlowCode = FlowCode(4008);
    // 41xx — other transfers (spec `economy/budget-2021`)
    /// Own resources paid to the EU budget (D76).
    pub const TRANSFER_EU_BUDGET: FlowCode = FlowCode(4101);
    /// Other current transfers and other taxes the government pays.
    pub const TRANSFER_OTHER_CURRENT: FlowCode = FlowCode(4102);
    /// Capital transfers (D9).
    pub const TRANSFER_CAPITAL: FlowCode = FlowCode(4103);
    // 5xxx — subsidies
    /// Subsidies on production (D3).
    pub const SUBSIDY_PRODUCTION: FlowCode = FlowCode(5001);
    // 7xxx — financial transactions
    /// Cash paid into a bank account (cash for deposits).
    pub const CASH_DEPOSIT: FlowCode = FlowCode(7001);
    /// Government bonds issued, bought, redeemed (at book value).
    pub const BONDS: FlowCode = FlowCode(7002);
    // 9xxx — opening balances and discrepancies
    /// Opening balance sheet entry.
    pub const OPENING: FlowCode = FlowCode(9001);

    /// Every code the ledger knows, in ascending order. A new code is added here
    /// and in [`FlowCode::name`] in the same change.
    pub const ALL: [FlowCode; 27] = [
        Self::CONSUMPTION,
        Self::GOV_PURCHASES,
        Self::EXPORTS,
        Self::IMPORTS,
        Self::WAGES,
        Self::INTEREST_GOV_DEBT,
        Self::MIXED_INCOME,
        Self::TAX_INCOME,
        Self::TAX_VAT,
        Self::TAX_EMPLOYER_CONTRIBUTION,
        Self::CONTRIB_PENSION,
        Self::CONTRIB_PENSION_PILLAR2,
        Self::CONTRIB_HEALTH,
        Self::TAX_PROFIT,
        Self::TAX_MICRO,
        Self::BENEFIT_PENSION,
        Self::BENEFIT_FAMILY,
        Self::BENEFIT_UNEMPLOYMENT,
        Self::BENEFIT_SOCIAL_ASSISTANCE,
        Self::BENEFIT_OTHER,
        Self::TRANSFER_EU_BUDGET,
        Self::TRANSFER_OTHER_CURRENT,
        Self::TRANSFER_CAPITAL,
        Self::SUBSIDY_PRODUCTION,
        Self::CASH_DEPOSIT,
        Self::BONDS,
        Self::OPENING,
    ];

    /// Whether this is one of the ledger's codes (AC-ACC-04).
    #[must_use]
    pub fn is_known(self) -> bool {
        Self::ALL.binary_search(&self).is_ok()
    }

    /// Human-readable name (for reports and the "why" panel).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self.0 {
            1001 => "consumption",
            1002 => "gov.purchases",
            1004 => "exports",
            1005 => "imports",
            2001 => "wages",
            2002 => "interest.gov_debt",
            2003 => "income.mixed",
            3001 => "tax.income",
            3002 => "tax.vat",
            3003 => "tax.employer_contribution",
            3004 => "contrib.pension",
            3005 => "contrib.pension_pillar2",
            3006 => "contrib.health",
            3007 => "tax.profit",
            3008 => "tax.micro",
            4001 => "benefit.pension",
            4003 => "benefit.family",
            4004 => "benefit.unemployment",
            4006 => "benefit.social_assistance",
            4008 => "benefit.other",
            4101 => "transfer.eu_budget",
            4102 => "transfer.other_current",
            4103 => "transfer.capital",
            5001 => "subsidy.production",
            7001 => "finance.cash_deposit",
            7002 => "finance.bonds",
            9001 => "opening",
            _ => "unknown",
        }
    }

    /// Whether this code belongs in the transaction-flow matrix (current and
    /// capital flows). Opening entries are stock adjustments, not flows.
    #[must_use]
    pub fn is_flow(self) -> bool {
        self.0 < 9000
    }
}
