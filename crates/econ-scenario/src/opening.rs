//! The opening balance sheet (spec `economy/opening-balance-sheet`): the
//! financial accounts of 31 December 2021 as the ledger's balances on the start
//! date, every instrument row summing to exactly zero. Pure functions of the
//! two input files, in integer bani; no I/O.
//!
//! **Skeleton** (ADR-0014): the API of the spec's sketch, written with the
//! acceptance tests; the bodies come with the implementation.

use econ_io::SaveError;
use econ_ledger::{Instrument, Ledger, LedgerError, Sector};
use econ_types::Bani;

/// Number of ledger sectors (columns of the accounting matrix).
pub const SECTORS: usize = 8;

/// An instrument of the financial accounts (`balance_sheets2021q4_ro.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatInstrument {
    /// Monetary gold and special drawing rights (`gold_and_sdr`).
    GoldAndSdr,
    /// Currency (`currency`).
    Currency,
    /// Deposits (`deposits`).
    Deposits,
    /// Debt securities (`debt_securities`).
    DebtSecurities,
    /// Loans (`loans`).
    Loans,
    /// Equity and investment fund shares (`equity`).
    Equity,
    /// Insurance, pension and standardised guarantee schemes (`insurance_and_pensions`).
    InsuranceAndPensions,
    /// Financial derivatives (`derivatives`); left out of the ledger in v1.
    Derivatives,
    /// Other accounts receivable and payable (`other_accounts`); left out in v1.
    OtherAccounts,
}

impl StatInstrument {
    /// Every statistical instrument, in the order of the fixture.
    pub const ALL: [StatInstrument; 9] = [
        StatInstrument::GoldAndSdr,
        StatInstrument::Currency,
        StatInstrument::Deposits,
        StatInstrument::DebtSecurities,
        StatInstrument::Loans,
        StatInstrument::Equity,
        StatInstrument::InsuranceAndPensions,
        StatInstrument::Derivatives,
        StatInstrument::OtherAccounts,
    ];

    /// The instruments the ledger does not hold in v1, in the order of
    /// [`OpeningBalances::left_out`].
    pub const LEFT_OUT: [StatInstrument; 2] =
        [StatInstrument::Derivatives, StatInstrument::OtherAccounts];

    /// The instrument's key in the fixture.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            StatInstrument::GoldAndSdr => "gold_and_sdr",
            StatInstrument::Currency => "currency",
            StatInstrument::Deposits => "deposits",
            StatInstrument::DebtSecurities => "debt_securities",
            StatInstrument::Loans => "loans",
            StatInstrument::Equity => "equity",
            StatInstrument::InsuranceAndPensions => "insurance_and_pensions",
            StatInstrument::Derivatives => "derivatives",
            StatInstrument::OtherAccounts => "other_accounts",
        }
    }
}

/// The financial accounts, in lei: `[instrument][sector]`, instruments in
/// [`StatInstrument::ALL`] order, sectors in `Sector::ALL` order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectorAccounts {
    /// What each sector holds.
    pub assets: Vec<[i64; SECTORS]>,
    /// What each sector owes.
    pub liabilities: Vec<[i64; SECTORS]>,
}

/// The ledger's opening balances.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpeningBalances {
    /// `[instrument][sector]` in bani: instruments in `Instrument::ALL`
    /// order, sectors in `Sector::ALL` order. Every row sums to zero.
    pub balances: Vec<[Bani; SECTORS]>,
    /// Net derivatives and other accounts by sector, in bani, in
    /// [`StatInstrument::LEFT_OUT`] order.
    pub left_out: Vec<[Bani; SECTORS]>,
    /// The statistical gap closed by rule 1, per [`StatInstrument::ALL`], in
    /// bani (assets minus liabilities before closing).
    pub gaps: Vec<Bani>,
}

/// Why the opening balance sheet could not be built (nothing is produced).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpeningError {
    /// The financial accounts lack an instrument.
    MissingInstrument(StatInstrument),
    /// The financial accounts lack a sector.
    MissingSector(Sector),
    /// A rule of the spec does not close on these data (rule 2: foreign notes
    /// exceed the vault cash; rule 4: deposits abroad differ from what the rest
    /// of the world owes; …).
    RuleDoesNotClose {
        /// The rule's number in the spec.
        rule: u8,
        /// The statistical instrument it maps.
        instrument: StatInstrument,
    },
    /// Rule 8: a household total is more than 10 millionths off the statistics.
    HouseholdsTooFar {
        /// `Cash` or `Deposits`.
        instrument: Instrument,
    },
    /// An instrument row does not sum to zero after the rules.
    RowNotZero(Instrument),
    /// The file is not a financial-accounts file.
    Parse(String),
}

/// Parse `balance_sheets2021q4_ro.json` (written by
/// `python/pipeline/normalise_balance_sheets.py`).
///
/// # Errors
/// [`OpeningError::MissingSector`] or [`OpeningError::MissingInstrument`] if a
/// sector or instrument of the ledger's mapping is not in the file;
/// [`OpeningError::Parse`] if it is not such a file.
pub fn read_sector_accounts(json: &str) -> Result<SectorAccounts, OpeningError> {
    let _ = json;
    unimplemented!("economy/opening-balance-sheet: read the financial accounts")
}

/// Rules 1 to 7: the ledger's opening balances from the financial accounts.
///
/// # Errors
/// A missing instrument, or a rule that does not close; nothing is produced.
pub fn opening_balances(accounts: &SectorAccounts) -> Result<OpeningBalances, OpeningError> {
    let _ = accounts;
    unimplemented!("economy/opening-balance-sheet: rules 1 to 7")
}

/// Rule 8: the household column from the population's weighted totals of
/// `hh_cash` and `hh_deposits`; the central bank absorbs the difference in
/// cash, the banks in deposits.
///
/// # Errors
/// [`OpeningError::HouseholdsTooFar`] if a total differs from the statistics'
/// figure by more than 10 millionths of it.
pub fn reconcile(
    opening: &OpeningBalances,
    household_cash: Bani,
    household_deposits: Bani,
) -> Result<OpeningBalances, OpeningError> {
    let _ = (opening, household_cash, household_deposits);
    unimplemented!("economy/opening-balance-sheet: rule 8")
}

impl OpeningBalances {
    /// The opening hash: FNV-1a 64 over the number of instruments and of
    /// sectors (u32 each, little-endian), then every balance (i64,
    /// little-endian), instrument by instrument, sector by sector.
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        unimplemented!("economy/opening-balance-sheet: the opening hash")
    }

    /// One linked transaction, flow code `opening`, that sets every balance of
    /// a ledger whose balances are zero (AC-OBS-06). Declares every negative
    /// opening balance as one the sector may hold.
    ///
    /// # Errors
    /// If the ledger rejects the transaction; nothing is applied.
    pub fn post(&self, ledger: &mut Ledger) -> Result<(), LedgerError> {
        let _ = ledger;
        unimplemented!("economy/opening-balance-sheet: post the balances")
    }

    /// The balance of one sector in one ledger instrument.
    pub fn balance(&self, instrument: Instrument, sector: Sector) -> Bani {
        let _ = (instrument, sector);
        unimplemented!("economy/opening-balance-sheet: look up a balance")
    }
}

/// Government debt by residual maturity and currency (`sector_details2021_ro.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernmentDebt {
    /// Total, EU methodology, nominal.
    pub total: Bani,
    /// Short-term debt (measured).
    pub short_term: Bani,
    /// Up to 1 year, 1–3, 3–5, 5–10 and over 10 years.
    pub by_residual_maturity: [Bani; 5],
    /// Lei, euro, dollar.
    pub by_currency: [Bani; 3],
}

/// The central bank's reserves by kind (`sector_details2021_ro.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CentralBankReserves {
    /// Monetary gold.
    pub gold: Bani,
    /// SDR holdings.
    pub sdr_holdings: Bani,
    /// Foreign currency deposits.
    pub fx_deposits: Bani,
    /// Foreign currency securities.
    pub fx_securities: Bani,
    /// Official foreign currency reserves.
    pub fx_reserves_official: Bani,
}

/// The sector details the ledger's later users need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectorDetails {
    /// Government debt.
    pub government_debt: GovernmentDebt,
    /// Central bank reserves.
    pub central_bank_reserves: CentralBankReserves,
}

/// Parse `sector_details2021_ro.json` (written by
/// `python/pipeline/normalise_sector_details.py`); amounts are lei × 100.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not such a file.
pub fn read_sector_details(json: &str) -> Result<SectorDetails, SaveError> {
    let _ = json;
    unimplemented!("economy/opening-balance-sheet: read the sector details")
}
