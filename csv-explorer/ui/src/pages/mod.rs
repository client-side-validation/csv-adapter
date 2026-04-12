/// Page components module.

pub mod home;
pub mod stats;
pub mod rights;
pub mod transfers;
pub mod seals;
pub mod wallet;
pub mod right_detail;
pub mod transfer_detail;
pub mod seal_detail;
pub mod contracts;
pub mod chains;

// Re-export all page components for use in routing
pub use home::Home;
pub use stats::Stats;
pub use rights::RightsList;
pub use transfers::TransfersList;
pub use seals::SealsList;
pub use wallet::Wallet;
pub use right_detail::RightDetail;
pub use transfer_detail::TransferDetail;
pub use seal_detail::SealDetail;
pub use contracts::ContractsList;
pub use chains::Chains;
