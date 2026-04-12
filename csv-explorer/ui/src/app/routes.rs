/// Routes module for the CSV Explorer UI.

use dioxus::prelude::*;
use dioxus_router::prelude::*;

use crate::pages::{Home, RightsList, TransfersList, SealsList, Stats, Wallet, RightDetail, TransferDetail, SealDetail, ContractsList, Chains};

#[derive(Routable, Clone, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/rights")]
    RightsList {},
    #[route("/rights/:id")]
    RightDetail { id: String },
    #[route("/transfers")]
    TransfersList {},
    #[route("/transfers/:id")]
    TransferDetail { id: String },
    #[route("/seals")]
    SealsList {},
    #[route("/seals/:id")]
    SealDetail { id: String },
    #[route("/contracts")]
    ContractsList {},
    #[route("/stats")]
    Stats {},
    #[route("/chains")]
    Chains {},
    #[route("/wallet")]
    Wallet {},
}
