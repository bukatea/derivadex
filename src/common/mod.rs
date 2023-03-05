use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use web3::types::{Address, H256};

#[derive(Copy, Clone, Deserialize, Serialize)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct Account {
    pub ddx_balance: Decimal,
    pub usd_balance: Decimal,
    pub trader_address: Address,

    #[serde(skip)]
    pub ddx_book_outstanding: Decimal,
    #[serde(skip)]
    pub usd_book_outstanding: Decimal,
}

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct Order {
    pub amount: Decimal,
    pub nonce: H256,
    pub price: Decimal,
    pub side: Side,
    pub trader_address: Address,

    #[serde(skip)]
    pub timestamp: u128,
}

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct Fill {
    pub maker_hash: H256,
    pub taker_hash: H256,
    pub fill_amount: Decimal,
    pub price: Decimal,
}
