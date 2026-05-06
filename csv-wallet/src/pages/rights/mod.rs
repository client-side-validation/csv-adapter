//! Sanads management pages.

pub mod consume;
pub mod create;
pub mod journey;
pub mod list;
pub mod show;
pub mod transfer;

pub use consume::ConsumeSanad;
pub use create::CreateSanad;
pub use journey::SanadJourney;
pub use list::Sanads;
pub use show::ShowSanad;
pub use transfer::TransferSanad;
