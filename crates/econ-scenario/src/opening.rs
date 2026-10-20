//! The opening balance sheet (spec `economy/opening-balance-sheet`): the
//! financial accounts of 31 December 2021 as the ledger's balances on the start
//! date, every instrument row summing to exactly zero. Pure functions of the
//! two input files, in integer bani; no I/O.
//!
//! Where the statistics give holders and issuers but not who holds whose
//! claim, the spec's rules decide; every other split is proportional, by
//! largest remainder with ties to the lower index, as in the reference
//! (`python/reference/opening_balance_reference.py`), so both agree to the ban.

use econ_io::SaveError;
use econ_ledger::{FlowCode, Instrument, Ledger, LedgerError, Sector, Txn};
use econ_types::Bani;
use serde_json::Value;

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

// Sector columns, in `Sector::ALL` order.
const HH: usize = 0;
const FIRMS: usize = 1;
const BANKS: usize = 2;
const PF: usize = 3;
const OF: usize = 4;
const GOV: usize = 5;
const CB: usize = 6;
const ROW: usize = 7;
/// Every sector but the rest of the world.
const RESIDENTS: [usize; 7] = [HH, FIRMS, BANKS, PF, OF, GOV, CB];

// Ledger instrument rows, in `Instrument::ALL` order.
const CASH: usize = 0;
const DEPOSITS: usize = 1;
const GOV_DEPOSITS_AT_CB: usize = 2;
const RESERVES: usize = 3;
const BONDS: usize = 4;
const LOANS: usize = 5;
const EQUITY: usize = 7;
const PENSION_CLAIMS: usize = 8;
const FOREIGN_ASSETS: usize = 9;
const INSTRUMENTS: usize = 10;

/// The fixture's key of each sector, in `Sector::ALL` order.
const SECTOR_KEYS: [&str; SECTORS] = [
    "households",
    "firms",
    "banks",
    "pension_funds",
    "other_financial",
    "government",
    "central_bank",
    "rest_of_world",
];

const BANI_PER_LEU: i64 = 100;

/// Rule 8: a household total may differ from the statistics by this many
/// millionths of the statistics' figure.
const HOUSEHOLD_TOLERANCE_PPM: i128 = 10;

fn parse(why: &str) -> OpeningError {
    OpeningError::Parse(why.to_string())
}

/// Parse `balance_sheets2021q4_ro.json` (written by
/// `python/pipeline/normalise_balance_sheets.py`).
///
/// # Errors
/// [`OpeningError::MissingSector`] or [`OpeningError::MissingInstrument`] if a
/// sector or instrument of the ledger's mapping is not in the file;
/// [`OpeningError::Parse`] if it is not such a file.
pub fn read_sector_accounts(json: &str) -> Result<SectorAccounts, OpeningError> {
    let v: Value = serde_json::from_str(json).map_err(|e| parse(&e.to_string()))?;
    let names = |key: &str| -> Result<Vec<&str>, OpeningError> {
        v.get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| parse(&format!("no list `{key}`")))?
            .iter()
            .map(|x| {
                x.as_str()
                    .ok_or_else(|| parse(&format!("`{key}` holds a non-string")))
            })
            .collect()
    };
    let (instruments, sectors) = (names("instruments")?, names("sectors")?);
    let side = |key: &str| {
        v.get(key)
            .and_then(Value::as_object)
            .ok_or_else(|| parse(&format!("no table `{key}`")))
    };
    let (assets, liabilities) = (side("assets")?, side("liabilities")?);
    for k in StatInstrument::ALL {
        let listed = instruments.contains(&k.key())
            && assets.contains_key(k.key())
            && liabilities.contains_key(k.key());
        if !listed {
            return Err(OpeningError::MissingInstrument(k));
        }
    }
    for (s, key) in Sector::ALL.into_iter().zip(SECTOR_KEYS) {
        let listed = sectors.contains(&key)
            && StatInstrument::ALL.iter().all(|k| {
                [assets, liabilities]
                    .iter()
                    .all(|t| t[k.key()].get(key).is_some())
            });
        if !listed {
            return Err(OpeningError::MissingSector(s));
        }
    }
    let table = |t: &serde_json::Map<String, Value>| -> Result<Vec<[i64; SECTORS]>, OpeningError> {
        StatInstrument::ALL
            .iter()
            .map(|k| {
                let mut row = [0i64; SECTORS];
                for (cell, key) in row.iter_mut().zip(SECTOR_KEYS) {
                    *cell = t[k.key()][key]
                        .as_i64()
                        .ok_or_else(|| parse(&format!("{} of {key} is not whole lei", k.key())))?;
                }
                Ok(row)
            })
            .collect()
    };
    Ok(SectorAccounts {
        assets: table(assets)?,
        liabilities: table(liabilities)?,
    })
}

/// `a // b` of Python: the quotient rounded toward minus infinity.
// `b` is never zero here.
#[allow(clippy::arithmetic_side_effects)]
fn floor_div(a: i128, b: i128) -> i128 {
    let q = a / b;
    if (a % b != 0) && ((a < 0) != (b < 0)) {
        q - 1
    } else {
        q
    }
}

/// `a % b` of Python: the remainder with the sign of `b`.
#[allow(clippy::arithmetic_side_effects)]
fn floor_mod(a: i128, b: i128) -> i128 {
    a - b * floor_div(a, b)
}

/// Split `total` in proportion to `targets` by largest remainder, ties to the
/// lower index; everything to the first if the targets sum to zero. The
/// reference's `apportion`.
// Amounts are bani below 10^15, so products stay far inside i128.
#[allow(clippy::arithmetic_side_effects)]
fn apportion(total: i64, targets: &[i64]) -> Vec<i64> {
    let tsum: i128 = targets.iter().map(|&t| i128::from(t)).sum();
    let total = i128::from(total);
    let mut parts = vec![0i128; targets.len()];
    if tsum == 0 {
        if let Some(first) = parts.first_mut() {
            *first = total;
        }
    } else {
        for (p, &t) in parts.iter_mut().zip(targets) {
            *p = floor_div(total * i128::from(t), tsum);
        }
        let mut order: Vec<usize> = (0..targets.len()).collect();
        order.sort_by_key(|&i| (-floor_mod(total * i128::from(targets[i]), tsum), i));
        let left = total - parts.iter().sum::<i128>();
        let n = i128::try_from(order.len()).expect("few targets");
        let take = if left >= 0 {
            left.min(n)
        } else {
            (n + left).max(0)
        };
        for &i in &order[..usize::try_from(take).expect("a count")] {
            parts[i] += 1;
        }
    }
    parts
        .into_iter()
        .map(|p| i64::try_from(p).expect("a part of an i64 total"))
        .collect()
}

const fn stat_row(k: StatInstrument) -> usize {
    k as usize
}

/// Rules 1 to 7: the ledger's opening balances from the financial accounts.
///
/// # Errors
/// A missing instrument, or a rule that does not close; nothing is produced.
// Amounts are bani of a national balance sheet, below 10^16: no sum of a
// few of them overflows i64.
#[allow(clippy::arithmetic_side_effects, clippy::too_many_lines)]
pub fn opening_balances(accounts: &SectorAccounts) -> Result<OpeningBalances, OpeningError> {
    for (n, k) in StatInstrument::ALL.into_iter().enumerate() {
        if accounts.assets.len() <= n || accounts.liabilities.len() <= n {
            return Err(OpeningError::MissingInstrument(k));
        }
    }
    use StatInstrument as S;
    // Rule 1: lei to bani, each instrument's gap closed on the rest of the world.
    let to_bani = |t: &[[i64; SECTORS]]| -> Vec<[i64; SECTORS]> {
        t.iter().map(|r| r.map(|v| v * BANI_PER_LEU)).collect()
    };
    let (mut a, mut l) = (to_bani(&accounts.assets), to_bani(&accounts.liabilities));
    let mut gaps = Vec::with_capacity(StatInstrument::ALL.len());
    for k in StatInstrument::ALL {
        let r = stat_row(k);
        let gap = a[r].iter().sum::<i64>() - l[r].iter().sum::<i64>();
        gaps.push(Bani(gap));
        if gap > 0 {
            l[r][ROW] += gap;
        } else if gap < 0 {
            a[r][ROW] -= gap;
        }
    }
    let mut b = vec![[0i64; SECTORS]; INSTRUMENTS];
    let fails =
        |rule: u8, instrument: StatInstrument| OpeningError::RuleDoesNotClose { rule, instrument };

    // Rule 2. Currency: the central bank's notes are cash; foreign notes are a
    // foreign asset of the central bank first, then of the banks' vaults.
    let (ca, cl) = (a[stat_row(S::Currency)], l[stat_row(S::Currency)]);
    let foreign = cl[ROW];
    let cb_foreign = ca[CB].min(foreign);
    let bank_foreign = foreign - cb_foreign;
    if bank_foreign > ca[BANKS] || RESIDENTS.iter().any(|&s| s != CB && cl[s] != 0) {
        return Err(fails(2, S::Currency));
    }
    for s in RESIDENTS {
        let held_foreign = match s {
            CB => cb_foreign,
            BANKS => bank_foreign,
            _ => 0,
        };
        b[CASH][s] += ca[s] - held_foreign;
        b[FOREIGN_ASSETS][s] += held_foreign;
    }
    b[CASH][CB] -= cl[CB];
    b[FOREIGN_ASSETS][ROW] += ca[ROW] - foreign;

    // Rule 4. Deposits.
    let (da, dl) = (a[stat_row(S::Deposits)], l[stat_row(S::Deposits)]);
    let gov_own = da[GOV].min(dl[GOV]);
    let gov_at_cb = da[GOV] - gov_own;
    let reserves = dl[CB] - gov_at_cb;
    if reserves < 0 || dl[GOV] > da[GOV] {
        return Err(fails(4, S::Deposits));
    }
    let at_banks = [HH, FIRMS, PF, OF, ROW];
    let interbank = dl[BANKS] - at_banks.iter().map(|&s| da[s]).sum::<i64>();
    let bank_abroad = da[BANKS] - reserves - interbank;
    if interbank < 0
        || bank_abroad < 0
        || da[CB] + bank_abroad != dl[ROW]
        || [HH, FIRMS, PF, OF].iter().any(|&s| dl[s] != 0)
    {
        return Err(fails(4, S::Deposits));
    }
    for s in at_banks {
        b[DEPOSITS][s] += da[s];
    }
    b[DEPOSITS][BANKS] += interbank - dl[BANKS];
    b[GOV_DEPOSITS_AT_CB][GOV] += gov_at_cb;
    b[GOV_DEPOSITS_AT_CB][CB] -= gov_at_cb;
    b[RESERVES][BANKS] += reserves;
    b[RESERVES][CB] -= reserves;
    b[FOREIGN_ASSETS][CB] += da[CB];
    b[FOREIGN_ASSETS][BANKS] += bank_abroad;
    b[FOREIGN_ASSETS][ROW] -= dl[ROW];

    // Rule 5. Debt securities.
    let (sa, sl) = (
        a[stat_row(S::DebtSecurities)],
        l[stat_row(S::DebtSecurities)],
    );
    let gov_own = sa[GOV].min(sl[GOV]);
    let cb_foreign = sa[CB];
    let other_foreign = sl[ROW] - cb_foreign;
    let row_gov = sa[ROW];
    if other_foreign < 0 || row_gov > sl[GOV] - gov_own {
        return Err(fails(5, S::DebtSecurities));
    }
    let private_issuers = [FIRMS, BANKS, PF, OF, CB];
    let private: i64 = private_issuers.iter().map(|&s| sl[s]).sum();
    let holders = [HH, FIRMS, BANKS, PF, OF];
    let foreign_holders = [BANKS, PF, OF];
    let held_foreign = apportion(other_foreign, &foreign_holders.map(|s| sa[s]));
    let held_foreign_of = |s: usize| {
        foreign_holders
            .iter()
            .position(|&f| f == s)
            .map_or(0, |i| held_foreign[i])
    };
    let rest = holders.map(|s| sa[s] - held_foreign_of(s));
    let held_private = apportion(private, &rest);
    for (i, &s) in holders.iter().enumerate() {
        b[FOREIGN_ASSETS][s] += held_foreign_of(s);
        b[LOANS][s] += held_private[i];
        b[BONDS][s] += rest[i] - held_private[i];
    }
    for s in private_issuers {
        b[LOANS][s] -= sl[s];
    }
    b[BONDS][ROW] += row_gov;
    b[BONDS][GOV] -= sl[GOV] - gov_own;
    b[FOREIGN_ASSETS][CB] += cb_foreign;
    b[FOREIGN_ASSETS][ROW] -= sl[ROW];
    if b[BONDS].iter().sum::<i64>() != 0 {
        return Err(fails(5, S::DebtSecurities));
    }

    // Rules 3 and 7: loans, insurance and pensions, after their cross-border part.
    let all_residents: &[usize] = &RESIDENTS;
    for (k, target, issuers, holders) in [
        (
            S::Loans,
            LOANS,
            &[FIRMS, BANKS, PF, OF, GOV][..],
            &[FIRMS, BANKS, PF, OF, GOV, CB][..],
        ),
        (
            S::InsuranceAndPensions,
            PENSION_CLAIMS,
            all_residents,
            all_residents,
        ),
    ] {
        let (ka, kl) = (a[stat_row(k)], l[stat_row(k)]);
        let mut owed = [0i64; SECTORS];
        let mut held = [0i64; SECTORS];
        let owed_parts = apportion(ka[ROW], &issuers.iter().map(|&s| kl[s]).collect::<Vec<_>>());
        let held_parts = apportion(kl[ROW], &holders.iter().map(|&s| ka[s]).collect::<Vec<_>>());
        for (&s, v) in issuers.iter().zip(owed_parts) {
            owed[s] = v;
        }
        for (&s, v) in holders.iter().zip(held_parts) {
            held[s] = v;
        }
        if RESIDENTS
            .iter()
            .any(|&s| owed[s] > kl[s] || held[s] > ka[s])
        {
            return Err(fails(3, k));
        }
        for s in RESIDENTS {
            b[target][s] += (ka[s] - held[s]) - (kl[s] - owed[s]);
            b[FOREIGN_ASSETS][s] += held[s] - owed[s];
        }
        b[FOREIGN_ASSETS][ROW] += ka[ROW] - kl[ROW];
    }

    // Rule 6. Equity: shares in residents stay equity; residents' shares
    // abroad are foreign assets, apportioned to their equity holdings.
    let (ea, el) = (a[stat_row(S::Equity)], l[stat_row(S::Equity)]);
    let held = apportion(el[ROW], &RESIDENTS.map(|s| ea[s]));
    for (i, &s) in RESIDENTS.iter().enumerate() {
        b[EQUITY][s] += ea[s] - held[i] - el[s];
        b[FOREIGN_ASSETS][s] += held[i];
    }
    b[EQUITY][ROW] += ea[ROW];
    b[FOREIGN_ASSETS][ROW] -= el[ROW];

    // Rule 7. Gold and SDR are foreign assets.
    let (ga, gl) = (a[stat_row(S::GoldAndSdr)], l[stat_row(S::GoldAndSdr)]);
    for s in 0..SECTORS {
        b[FOREIGN_ASSETS][s] += ga[s] - gl[s];
    }

    for (r, i) in Instrument::ALL.into_iter().enumerate() {
        if b[r].iter().sum::<i64>() != 0 {
            return Err(OpeningError::RowNotZero(i));
        }
    }
    let left_out = StatInstrument::LEFT_OUT
        .iter()
        .map(|&k| {
            let r = stat_row(k);
            std::array::from_fn(|s| Bani(a[r][s] - l[r][s]))
        })
        .collect();
    Ok(OpeningBalances {
        balances: b.into_iter().map(|r| r.map(Bani)).collect(),
        left_out,
        gaps,
    })
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
    let mut out = opening.clone();
    for (instrument, total, issuer) in [
        (Instrument::Cash, household_cash, CB),
        (Instrument::Deposits, household_deposits, BANKS),
    ] {
        let r = &mut out.balances[instrument as usize];
        let diff = total - r[HH];
        let off = i128::from(diff.get()).abs().saturating_mul(1_000_000);
        if off > HOUSEHOLD_TOLERANCE_PPM.saturating_mul(i128::from(r[HH].get()).abs()) {
            return Err(OpeningError::HouseholdsTooFar { instrument });
        }
        r[HH] = total;
        r[issuer] -= diff;
    }
    Ok(out)
}

impl OpeningBalances {
    /// The opening hash: FNV-1a 64 over the number of instruments and of
    /// sectors (u32 each, little-endian), then every balance (i64,
    /// little-endian), instrument by instrument, sector by sector.
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut write = |bytes: &[u8]| {
            for &byte in bytes {
                h = (h ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        let n = |x: usize| u32::try_from(x).expect("a small count").to_le_bytes();
        write(&n(self.balances.len()));
        write(&n(SECTORS));
        for row in &self.balances {
            for b in row {
                write(&b.get().to_le_bytes());
            }
        }
        h
    }

    /// One linked transaction, flow code `opening`, that sets every balance of
    /// a ledger whose balances are zero (AC-OBS-06). Declares every negative
    /// opening balance as one the sector may hold.
    ///
    /// Each row is walked in sector order with its running sum: the leg
    /// between a sector and the next moves that sum, so each sector ends at
    /// its balance and the row sums to zero after every leg.
    ///
    /// # Errors
    /// If the ledger rejects the transaction; nothing is applied.
    pub fn post(&self, ledger: &mut Ledger) -> Result<(), LedgerError> {
        let mut txn = Txn::new();
        for (instrument, row) in Instrument::ALL.into_iter().zip(&self.balances) {
            let mut carried = Bani::ZERO;
            for (s, &balance) in row.iter().enumerate() {
                if balance < Bani::ZERO {
                    ledger.allow_negative(Sector::ALL[s], instrument);
                }
                carried += balance;
                let Some(&next) = Sector::ALL.get(s.saturating_add(1)) else {
                    break;
                };
                let here = Sector::ALL[s];
                if carried > Bani::ZERO {
                    txn = txn.leg(next, here, instrument, carried, FlowCode::OPENING);
                } else if carried < Bani::ZERO {
                    txn = txn.leg(here, next, instrument, -carried, FlowCode::OPENING);
                }
            }
        }
        ledger.commit(txn)
    }

    /// The balance of one sector in one ledger instrument.
    pub fn balance(&self, instrument: Instrument, sector: Sector) -> Bani {
        self.balances[instrument as usize][sector as usize]
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
    let v: Value = serde_json::from_str(json).map_err(|e| SaveError::Parse(e.to_string()))?;
    let amount = |path: &[&str]| -> Result<Bani, SaveError> {
        let mut x = &v;
        for key in path {
            x = x
                .get(key)
                .ok_or_else(|| SaveError::Parse(format!("no `{}`", path.join("."))))?;
        }
        let lei = x
            .as_i64()
            .ok_or_else(|| SaveError::Parse(format!("`{}` is not whole lei", path.join("."))))?;
        lei.checked_mul(BANI_PER_LEU)
            .map(Bani)
            .ok_or_else(|| SaveError::Parse(format!("`{}` is too large", path.join("."))))
    };
    let debt = |key: &str| amount(&["government_debt", key]);
    let bucket = |group: &str, key: &str| amount(&["government_debt", group, key]);
    let reserve = |key: &str| amount(&["central_bank_reserves", key]);
    Ok(SectorDetails {
        government_debt: GovernmentDebt {
            total: debt("total")?,
            short_term: debt("short_term")?,
            by_residual_maturity: [
                bucket("by_residual_maturity", "up_to_1y")?,
                bucket("by_residual_maturity", "1_to_3y")?,
                bucket("by_residual_maturity", "3_to_5y")?,
                bucket("by_residual_maturity", "5_to_10y")?,
                bucket("by_residual_maturity", "over_10y")?,
            ],
            by_currency: [
                bucket("by_currency", "lei")?,
                bucket("by_currency", "eur")?,
                bucket("by_currency", "usd")?,
            ],
        },
        central_bank_reserves: CentralBankReserves {
            gold: reserve("gold")?,
            sdr_holdings: reserve("sdr_holdings")?,
            fx_deposits: reserve("fx_deposits")?,
            fx_securities: reserve("fx_securities")?,
            fx_reserves_official: reserve("fx_reserves_official")?,
        },
    })
}
