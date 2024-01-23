use serde::{Deserialize, Serialize};

use crate::Snowflake;

/// Job to download a work by its ID.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct NewWorkJob {
    pub id: Snowflake,
    pub work_id: u64,
}

impl NewWorkJob {
    pub fn new(work_id: u64) -> Self {
        Self {
            id: Snowflake::new_random(),
            work_id,
        }
    }
}
