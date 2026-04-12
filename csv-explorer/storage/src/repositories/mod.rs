/// Repository modules for database access.

pub mod contracts;
pub mod rights;
pub mod seals;
pub mod stats;
pub mod sync;
pub mod transfers;

pub use contracts::ContractsRepository;
pub use rights::RightsRepository;
pub use seals::SealsRepository;
pub use stats::StatsRepository;
pub use sync::SyncRepository;
pub use transfers::TransfersRepository;
