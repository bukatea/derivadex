use displaydoc::Display;
use thiserror::Error;
use web3::types::{Address, H256};

pub type Result<T> = std::result::Result<T, OrderBookError>;

#[derive(Debug, Display, Error)]
pub enum OrderBookError {
    /// duplicate order submitted with hash {0} by account {1}
    DuplicateOrder(H256, Address),

    /// order with hash {0} not found,
    OrderNotFound(H256),
}
