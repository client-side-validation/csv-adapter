//! Rights management pages.

pub mod list;
pub mod create;
pub mod show;
pub mod transfer;
pub mod consume;
pub mod journey;

pub use list::Rights;
pub use create::{CreateRight, CreateRightForm};
pub use show::ShowRight;
pub use transfer::TransferRight;
pub use consume::ConsumeRight;
pub use journey::RightJourney;
