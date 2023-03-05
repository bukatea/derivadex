use displaydoc::Display;
use rust_decimal::Decimal;
use thiserror::Error;
use web3::types::Address;

use super::orderbook::OrderBookError;

pub type Result<T> = std::result::Result<T, EngineError>;

#[derive(Debug, Display, Error)]
pub enum EngineError {
    /// negative balance {0} rejected
    NegativeBalance(Decimal),

    /// account with address {0} already exists
    AccountAlreadyExists(Address),

    /// account with address {0} not found
    AccountNotFound(Address),

    /// insufficient balance {0} for order cost {1}
    InsufficientBalance(Decimal, Decimal),

    /// orderbook error: {0}
    OrderBookError(#[from] OrderBookError),
}
