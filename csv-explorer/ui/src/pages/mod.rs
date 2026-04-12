/// Page components module.

pub mod chains;
pub mod contracts;
pub mod home;
pub mod right_detail;
pub mod rights;
pub mod seal_detail;
pub mod seals;
pub mod stats;
pub mod transfer_detail;
pub mod transfers;
pub mod wallet;

// Re-export all page components for use in routing
pub use chains::Chains;
pub use contracts::ContractsList;
pub use home::Home;
pub use right_detail::RightDetail;
pub use rights::RightsList;
pub use seal_detail::SealDetail;
pub use seals::SealsList;
pub use stats::Stats;
pub use transfer_detail::TransferDetail;
pub use transfers::TransfersList;
pub use wallet::Wallet;
