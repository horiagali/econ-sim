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
    /// Government fixed investment and land (P5, NP).
    pub const GOV_INVESTMENT: FlowCode = FlowCode(1003);
    /// Exports: the rest of the world pays for goods (M2: in deposits).
    pub const EXPORTS: FlowCode = FlowCode(1004);
    /// Imports: residents pay the rest of the world for goods, inputs
    /// included (M2: in deposits, at the starting price).
    pub const IMPORTS: FlowCode = FlowCode(1005);
    /// The public units' market sales, which go to the government (ESA
    /// P11 + P12 + P131, spec `game/lever-system`, "Market sales").
    pub const GOV_SALES: FlowCode = FlowCode(1006);
    // 2xxx — income
    /// Wages paid by firms.
    pub const WAGES: FlowCode = FlowCode(2001);
    /// Interest on government debt (D4), to its holders.
    pub const INTEREST_GOV_DEBT: FlowCode = FlowCode(2002);
    /// Interest borrowers pay the banks on their loans.
    pub const INTEREST_LOAN: FlowCode = FlowCode(2004);
    /// Interest the banks pay depositors.
    pub const INTEREST_DEPOSIT: FlowCode = FlowCode(2005);
    /// Interest the central bank pays on excess reserves and charges on
    /// advances.
    pub const INTEREST_RESERVES: FlowCode = FlowCode(2006);
    /// What the central bank's foreign assets earn, paid by the rest of the
    /// world.
    pub const INTEREST_CB_FOREIGN: FlowCode = FlowCode(2007);
    /// Dividends firm units pay their owners.
    pub const INCOME_DIVIDEND: FlowCode = FlowCode(2010);
    /// Rent and royalties on natural resources paid to the government.
    pub const RENT_NATURAL_RESOURCES: FlowCode = FlowCode(2011);
    /// Interest the government receives (held at 2021 levels).
    pub const INTEREST_GOV_RECEIVED: FlowCode = FlowCode(2012);
    /// Owner-occupiers' imputed rent paid back by the real-estate units.
    pub const INCOME_IMPUTED_RENT: FlowCode = FlowCode(2013);
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
    /// The tax on dividends, withheld by the paying unit.
    pub const TAX_DIVIDEND: FlowCode = FlowCode(3009);
    /// The tax on households' deposit interest, withheld by the banks.
    pub const TAX_INTEREST: FlowCode = FlowCode(3010);
    /// Excise duties on fuel, tobacco and alcohol.
    pub const TAX_EXCISE: FlowCode = FlowCode(3011);
    /// Taxes on products that firms pay on their inputs and investment.
    pub const TAX_PRODUCTS: FlowCode = FlowCode(3012);
    /// Other taxes on production (D29).
    pub const TAX_PRODUCTION: FlowCode = FlowCode(3013);
    /// Households' taxes on property and vehicles.
    pub const TAX_PROPERTY: FlowCode = FlowCode(3014);
    /// The tax on gambling winnings.
    pub const TAX_GAMBLING: FlowCode = FlowCode(3015);
    /// The tax on households' net worth above a threshold (a lever; none in
    /// 2021).
    pub const TAX_WEALTH: FlowCode = FlowCode(3016);
    // 4xxx — social transfers (numbering of spec `economy/budget-2021`)
    /// Old-age and survivors' pensions.
    pub const BENEFIT_PENSION: FlowCode = FlowCode(4001);
    /// Sickness and disability benefits.
    pub const BENEFIT_DISABILITY: FlowCode = FlowCode(4002);
    /// Child allowance.
    pub const BENEFIT_FAMILY: FlowCode = FlowCode(4003);
    /// Unemployment benefit.
    pub const BENEFIT_UNEMPLOYMENT: FlowCode = FlowCode(4004);
    /// Housing benefit.
    pub const BENEFIT_HOUSING: FlowCode = FlowCode(4005);
    /// Guaranteed minimum income.
    pub const BENEFIT_SOCIAL_ASSISTANCE: FlowCode = FlowCode(4006);
    /// Grants to pupils and students.
    pub const BENEFIT_STUDENT: FlowCode = FlowCode(4007);
    /// Other social protection (and one-off transfers).
    pub const BENEFIT_OTHER: FlowCode = FlowCode(4008);
    /// Social transfers in kind bought from market producers (D632).
    pub const BENEFIT_IN_KIND: FlowCode = FlowCode(4009);
    // 41xx — other transfers (spec `economy/budget-2021`)
    /// Own resources paid to the EU budget (D76).
    pub const TRANSFER_EU_BUDGET: FlowCode = FlowCode(4101);
    /// Other current transfers and other taxes the government pays.
    pub const TRANSFER_OTHER_CURRENT: FlowCode = FlowCode(4102);
    /// Capital transfers (D9).
    pub const TRANSFER_CAPITAL: FlowCode = FlowCode(4103);
    /// The central bank's profit remitted to the government.
    pub const TRANSFER_CB_PROFIT: FlowCode = FlowCode(4104);
    /// The EU's current grants to the government.
    pub const TRANSFER_EU_GRANTS_CURRENT: FlowCode = FlowCode(4105);
    /// The EU's capital grants to the government (capital account).
    pub const TRANSFER_EU_GRANTS_CAPITAL: FlowCode = FlowCode(4106);
    /// Fines, penalties and other current transfers to the government.
    pub const TRANSFER_OTHER_RECEIVED: FlowCode = FlowCode(4107);
    /// Capital transfers to the government, other than the EU's.
    pub const TRANSFER_CAPITAL_RECEIVED: FlowCode = FlowCode(4108);
    /// Estates without an heir, and the remainders of estates, to the
    /// government (a capital transfer; spec economy/demographics).
    pub const TRANSFER_ESTATE: FlowCode = FlowCode(4109);
    /// Remittances: personal transfers from linked emigrants abroad to their
    /// households (secondary income, spec economy/migration-remittances).
    pub const TRANSFER_REMITTANCES: FlowCode = FlowCode(4110);
    /// The net deposits a household takes when it emigrates whole, from
    /// households to the rest of the world (capital account, spec
    /// economy/migration-remittances).
    pub const TRANSFER_MIGRANT_CAPITAL: FlowCode = FlowCode(4111);
    // 5xxx — subsidies
    /// Subsidies on production (D3).
    pub const SUBSIDY_PRODUCTION: FlowCode = FlowCode(5001);
    /// The EU's direct payments to farms.
    pub const SUBSIDY_EU_AGRI: FlowCode = FlowCode(5002);
    // 7xxx — financial transactions
    /// Cash paid into a bank account (cash for deposits).
    pub const CASH_DEPOSIT: FlowCode = FlowCode(7001);
    /// Government bonds issued, bought, redeemed (at book value).
    pub const BONDS: FlowCode = FlowCode(7002);
    /// A bank loan drawn or repaid (a loan creates a deposit).
    pub const LOAN: FlowCode = FlowCode(7003);
    /// The banks' loss on a loan written off.
    pub const WRITE_OFF: FlowCode = FlowCode(7004);
    /// Central bank advances to the banks, drawn or repaid.
    pub const CB_ADVANCE: FlowCode = FlowCode(7005);
    /// The central bank sells (or buys) foreign assets against the rest of
    /// the world's lei deposits.
    pub const FX_INTERVENTION: FlowCode = FlowCode(7006);
    // 9xxx — opening balances and discrepancies
    /// Opening balance sheet entry.
    pub const OPENING: FlowCode = FlowCode(9001);

    /// Every code the ledger knows, in ascending order. A new code is added here
    /// and in [`FlowCode::name`] in the same change.
    pub const ALL: [FlowCode; 62] = [
        Self::CONSUMPTION,
        Self::GOV_PURCHASES,
        Self::GOV_INVESTMENT,
        Self::EXPORTS,
        Self::IMPORTS,
        Self::GOV_SALES,
        Self::WAGES,
        Self::INTEREST_GOV_DEBT,
        Self::MIXED_INCOME,
        Self::INTEREST_LOAN,
        Self::INTEREST_DEPOSIT,
        Self::INTEREST_RESERVES,
        Self::INTEREST_CB_FOREIGN,
        Self::INCOME_DIVIDEND,
        Self::RENT_NATURAL_RESOURCES,
        Self::INTEREST_GOV_RECEIVED,
        Self::INCOME_IMPUTED_RENT,
        Self::TAX_INCOME,
        Self::TAX_VAT,
        Self::TAX_EMPLOYER_CONTRIBUTION,
        Self::CONTRIB_PENSION,
        Self::CONTRIB_PENSION_PILLAR2,
        Self::CONTRIB_HEALTH,
        Self::TAX_PROFIT,
        Self::TAX_MICRO,
        Self::TAX_DIVIDEND,
        Self::TAX_INTEREST,
        Self::TAX_EXCISE,
        Self::TAX_PRODUCTS,
        Self::TAX_PRODUCTION,
        Self::TAX_PROPERTY,
        Self::TAX_GAMBLING,
        Self::TAX_WEALTH,
        Self::BENEFIT_PENSION,
        Self::BENEFIT_DISABILITY,
        Self::BENEFIT_FAMILY,
        Self::BENEFIT_UNEMPLOYMENT,
        Self::BENEFIT_HOUSING,
        Self::BENEFIT_SOCIAL_ASSISTANCE,
        Self::BENEFIT_STUDENT,
        Self::BENEFIT_OTHER,
        Self::BENEFIT_IN_KIND,
        Self::TRANSFER_EU_BUDGET,
        Self::TRANSFER_OTHER_CURRENT,
        Self::TRANSFER_CAPITAL,
        Self::TRANSFER_CB_PROFIT,
        Self::TRANSFER_EU_GRANTS_CURRENT,
        Self::TRANSFER_EU_GRANTS_CAPITAL,
        Self::TRANSFER_OTHER_RECEIVED,
        Self::TRANSFER_CAPITAL_RECEIVED,
        Self::TRANSFER_ESTATE,
        Self::TRANSFER_REMITTANCES,
        Self::TRANSFER_MIGRANT_CAPITAL,
        Self::SUBSIDY_PRODUCTION,
        Self::SUBSIDY_EU_AGRI,
        Self::CASH_DEPOSIT,
        Self::BONDS,
        Self::LOAN,
        Self::WRITE_OFF,
        Self::CB_ADVANCE,
        Self::FX_INTERVENTION,
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
            1003 => "gov.investment",
            1004 => "exports",
            1005 => "imports",
            1006 => "gov.sales",
            2001 => "wages",
            2002 => "interest.gov_debt",
            2004 => "interest.loan",
            2005 => "interest.deposit",
            2006 => "interest.reserves",
            2007 => "interest.cb_foreign",
            2010 => "income.dividend",
            2011 => "rent.natural_resources",
            2012 => "interest.gov_received",
            2013 => "income.imputed_rent",
            2003 => "income.mixed",
            3001 => "tax.income",
            3002 => "tax.vat",
            3003 => "tax.employer_contribution",
            3004 => "contrib.pension",
            3005 => "contrib.pension_pillar2",
            3006 => "contrib.health",
            3007 => "tax.profit",
            3008 => "tax.micro",
            3009 => "tax.dividend",
            3010 => "tax.interest",
            3011 => "tax.excise",
            3012 => "tax.products",
            3013 => "tax.production",
            3014 => "tax.property",
            3015 => "tax.gambling",
            3016 => "tax.wealth",
            4001 => "benefit.pension",
            4002 => "benefit.disability",
            4003 => "benefit.family",
            4004 => "benefit.unemployment",
            4005 => "benefit.housing",
            4006 => "benefit.social_assistance",
            4007 => "benefit.student",
            4008 => "benefit.other",
            4009 => "benefit.in_kind",
            4101 => "transfer.eu_budget",
            4102 => "transfer.other_current",
            4103 => "transfer.capital",
            4104 => "transfer.cb_profit",
            4105 => "transfer.eu_grants_current",
            4106 => "transfer.eu_grants_capital",
            4107 => "transfer.other_received",
            4108 => "transfer.capital_received",
            4109 => "transfer.estate",
            4110 => "transfer.remittances",
            4111 => "transfer.migrant_capital",
            5001 => "subsidy.production",
            5002 => "subsidy.eu_agri",
            7001 => "finance.cash_deposit",
            7002 => "finance.bonds",
            7003 => "finance.loan",
            7004 => "finance.write_off",
            7005 => "finance.cb_advance",
            7006 => "finance.fx_intervention",
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
