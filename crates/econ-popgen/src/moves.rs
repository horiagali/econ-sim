//! The inputs of moves between counties (spec `economy/migration-remittances`,
//! step 6): data only, read by the `World`'s migration.

/// Each county's 2021 net balance of changes of domicile and its seat's
/// position, in the scenario's county order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CountyMoves {
    /// Net balance of changes of domicile in 2021, persons (sums to zero).
    pub net_2021: Vec<i64>,
    /// The county seat's latitude, thousandths of a degree.
    pub seat_lat_milli: Vec<i32>,
    /// The county seat's longitude, thousandths of a degree.
    pub seat_lon_milli: Vec<i32>,
    /// Every change of domicile between localities in 2021, persons.
    pub settlements_2021: u64,
    /// `between_county_share`: the share of those crossing a county border,
    /// millionths.
    pub between_county_share_ppm: u32,
}
