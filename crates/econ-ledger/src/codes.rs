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
    // 2xxx — income
    /// Wages paid by firms.
    pub const WAGES: FlowCode = FlowCode(2001);
    // 3xxx — taxes
    /// Personal income tax.
    pub const TAX_INCOME: FlowCode = FlowCode(3001);
    /// Value-added tax collected on household purchases.
    pub const TAX_VAT: FlowCode = FlowCode(3002);
    // 9xxx — opening balances and discrepancies
    /// Opening balance sheet entry.
    pub const OPENING: FlowCode = FlowCode(9001);

    /// Human-readable name (for reports and the "why" panel).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self.0 {
            1001 => "consumption",
            1002 => "gov.purchases",
            2001 => "wages",
            3001 => "tax.income",
            3002 => "tax.vat",
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
