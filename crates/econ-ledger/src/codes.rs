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
    /// Self-employment and informal income paid out of firm units' surplus
    /// to households (M2 placeholder of spec `economy/households`).
    pub const MIXED_INCOME: FlowCode = FlowCode(2003);
    // 3xxx — taxes
    /// Personal income tax.
    pub const TAX_INCOME: FlowCode = FlowCode(3001);
    /// Value-added tax collected on household purchases.
    pub const TAX_VAT: FlowCode = FlowCode(3002);
    // 4xxx — social transfers (numbering of spec `economy/budget-2021`)
    /// Old-age and survivors' pensions.
    pub const BENEFIT_PENSION: FlowCode = FlowCode(4001);
    /// Other social protection (and one-off transfers).
    pub const BENEFIT_OTHER: FlowCode = FlowCode(4008);
    // 7xxx — financial transactions
    /// Cash paid into a bank account (cash for deposits).
    pub const CASH_DEPOSIT: FlowCode = FlowCode(7001);
    // 9xxx — opening balances and discrepancies
    /// Opening balance sheet entry.
    pub const OPENING: FlowCode = FlowCode(9001);

    /// Every code the ledger knows, in ascending order. A new code is added here
    /// and in [`FlowCode::name`] in the same change.
    pub const ALL: [FlowCode; 12] = [
        Self::CONSUMPTION,
        Self::GOV_PURCHASES,
        Self::EXPORTS,
        Self::IMPORTS,
        Self::WAGES,
        Self::MIXED_INCOME,
        Self::TAX_INCOME,
        Self::TAX_VAT,
        Self::BENEFIT_PENSION,
        Self::BENEFIT_OTHER,
        Self::CASH_DEPOSIT,
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
            2003 => "income.mixed",
            3001 => "tax.income",
            3002 => "tax.vat",
            4001 => "benefit.pension",
            4008 => "benefit.other",
            7001 => "finance.cash_deposit",
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
