//! Integer ID newtypes. IDs index column tables; they are never pointers.

macro_rules! id_type {
    ($(#[$m:meta])* $name:ident, $inner:ty) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct $name(pub $inner);

        impl $name {
            /// Index into a column table.
            #[inline]
            #[must_use]
            pub fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

id_type!(/// A synthetic person record.
    PersonId, u32);
id_type!(/// A synthetic household record.
    HouseholdId, u32);
id_type!(/// A firm unit (named firm or size-class cohort, ADR-0016).
    FirmId, u32);
id_type!(/// One of the ~90 industries.
    IndustryId, u16);
id_type!(/// One of Romania's 42 counties (41 + Bucharest).
    CountyId, u8);

/// One simulation tick. 1 tick = 1 month; month 0 is the scenario start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Month(pub u32);

impl Month {
    /// The following month.
    #[must_use]
    pub fn next(self) -> Self {
        Month(self.0.checked_add(1).expect("Month overflow"))
    }
}

#[cfg(test)]
mod glossary_tests {
    #[test]
    fn glossary_registry_is_populated_and_unique() {
        let v = crate::glossary::VARIABLES;
        assert!(v.len() > 50);
        for (i, a) in v.iter().enumerate() {
            assert!(
                v.iter().skip(i + 1).all(|b| b.code_name != a.code_name),
                "duplicate {}",
                a.code_name
            );
        }
        assert!(crate::glossary::lookup("hh_weight").is_some());
    }
}
