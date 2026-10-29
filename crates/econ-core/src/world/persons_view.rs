//! Person columns the M5 criteria read (features `migration`, `education`,
//! `approval`, `poverty`): each person's age and main activity, which the
//! `World` keeps but did not show.
//!
//! Test writer's addition (ADR-0014), written with those tests before the
//! code; the bodies are `unimplemented!` until the implementation fills them.

use econ_popgen::Activity;

use super::World;

impl World {
    /// Each person's age in months, aligned with [`super::PersonColumns`].
    #[must_use]
    pub fn person_age_months(&self) -> &[u16] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// Each person's main activity (Stage C's, kept by the `World`: a job
    /// seeker is `Unemployed`), aligned with [`super::PersonColumns`].
    #[must_use]
    pub fn activity(&self) -> &[Activity] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}
