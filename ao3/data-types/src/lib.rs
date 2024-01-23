pub mod snowflake;
pub use snowflake::*;
pub mod jobs;
pub use jobs::*;

pub const USER_AGENT: &'static str = concat!(
    "ao3-scraper-rs/",
    env!("CARGO_PKG_VERSION"),
    ", +https://github.com/danya02/ao3-scraper-rs"
);
