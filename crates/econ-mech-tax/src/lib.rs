//! Taxation mechanics (spec `economy/taxation`): VAT (spec `economy/vat`) and
//! the taxes on income and wages of the first version (`income`).

pub mod income;
// Excise duties (spec `economy/excise`): declared by the test writer from the
// spec's API sketch (ADR-0014); the implementation fills it and adds the
// feature to `default`.
#[cfg(feature = "excise")]
pub mod excise;
pub mod vat;
