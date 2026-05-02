//! Contract management pages.

pub mod add;
pub mod deploy;
pub mod list;
pub mod modal;
pub mod status;

pub use add::AddContract;
pub use deploy::DeployContract;
pub use list::Contracts;
pub use modal::ContractDetailModal;
pub use status::ContractStatus;
