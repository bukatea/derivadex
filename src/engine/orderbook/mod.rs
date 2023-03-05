mod error;
pub use error::OrderBookError;
use error::{OrderBookError as Error, Result};

mod eip712;
use eip712::{Eip712, Eip712Domain, EncodeDataable, TypeHashable};

use lazy_static::lazy_static;
use rust_decimal::Decimal;
use serde::Serialize;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap},
    ops::Bound::{Included, Unbounded},
};
use web3::{
    signing::keccak256,
    types::{H256, U256},
};

use crate::{Fill, Order, Side};

fn decimal_to_u256(decimal: Decimal) -> U256 {
    U256::from_dec_str(&decimal.to_string()).unwrap()
}

lazy_static! {
    static ref ORDER_HASH: [u8; 32] = keccak256(
        "Order(uint256 amount,uint256 nonce,uint256 price,uint8 side,address traderAddress)"
            .as_bytes()
    );
}

impl TypeHashable for Order {
    fn type_hash(&self) -> [u8; 32] {
        *ORDER_HASH
    }
}

impl EncodeDataable for Order {
    fn encode_data(&self) -> Vec<u8> {
        [
            decimal_to_u256(self.amount).encode_data(),
            Into::<U256>::into(self.nonce.to_fixed_bytes()).encode_data(),
            decimal_to_u256(self.price).encode_data(),
            match self.side {
                Side::Bid => 0u8,
                Side::Ask => 1u8,
            }
            .encode_data(),
            self.trader_address.encode_data(),
        ]
        .concat()
    }
}

#[derive(Copy, Clone, Serialize)]
pub struct L2Order {
    amount: Decimal,
    price: Decimal,
}

#[derive(Clone, Serialize)]
pub struct L2OrderBook {
    asks: Vec<L2Order>,
    bids: Vec<L2Order>,
}

pub struct OrderBook {
    asks: BTreeMap<(Decimal, u128), Order>,
    bids: BTreeMap<(Reverse<Decimal>, u128), Order>,
    // could have used Rc<RefCell<Order>> here, but seems unnecessary since Order is Copy
    // may be wrong
    hash_to_order: HashMap<H256, Order>,
    eip712: Eip712,

    // ordered map from price level to amount
    // iter().take(n) is very slow, and this is a small tradeoff of space
    agg_ask_amt: BTreeMap<Decimal, Decimal>,
    agg_bid_amt: BTreeMap<Reverse<Decimal>, Decimal>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
            hash_to_order: HashMap::new(),
            eip712: Eip712::new(Eip712Domain {
                name: "DDX take-home",
                version: "0.1.0",
            }),
            agg_ask_amt: BTreeMap::new(),
            agg_bid_amt: BTreeMap::new(),
        }
    }

    pub fn add_bid(&mut self, mut bid: Order) -> Result<Vec<Fill>> {
        let taker_hash = self.eip712.encode(bid);
        if let Some(existing_bid) = self.hash_to_order.get(&taker_hash) {
            if existing_bid.trader_address == bid.trader_address {
                return Err(Error::DuplicateOrder(taker_hash, bid.trader_address));
            }
        }

        // get possible fills
        let mut fills = vec![];
        for (_, ask) in self
            .asks
            .range((Unbounded, Included((bid.price, bid.timestamp))))
        {
            if ask.trader_address == bid.trader_address {
                // self-match
                break;
            }

            // TODO: save so don't have to recompute
            let maker_hash = self.eip712.encode(*ask);
            let fill_amount = bid.amount.min(ask.amount);
            let fill = Fill {
                maker_hash,
                taker_hash,
                fill_amount,
                price: ask.price,
            };
            fills.push(fill);
            bid.amount -= fill_amount;
            if bid.amount == Decimal::ZERO {
                break;
            }
        }

        // update book to reflect fills
        for fill in &fills {
            let ask = self.hash_to_order[&fill.maker_hash];
            if ask.amount == fill.fill_amount {
                // fill completely uses up ask, remove
                self.asks.remove(&(ask.price, ask.timestamp));
                self.hash_to_order.remove(&fill.maker_hash);
                *self.agg_ask_amt.get_mut(&ask.price).unwrap() -= ask.amount;
                if self.agg_ask_amt[&ask.price] == Decimal::ZERO {
                    self.agg_ask_amt.remove(&ask.price);
                }
            } else {
                self.asks
                    .get_mut(&(ask.price, ask.timestamp))
                    .unwrap()
                    .amount -= fill.fill_amount;
                *self.agg_ask_amt.get_mut(&ask.price).unwrap() -= ask.amount;
            }
        }

        if bid.amount > Decimal::ZERO {
            // add remaining bid to book
            self.bids.insert((Reverse(bid.price), bid.timestamp), bid);
            self.hash_to_order
                .insert(taker_hash, self.bids[&(Reverse(bid.price), bid.timestamp)]);
            *self
                .agg_bid_amt
                .entry(Reverse(bid.price))
                .or_insert(Decimal::ZERO) += bid.amount;
        }

        Ok(fills)
    }

    pub fn add_ask(&mut self, mut ask: Order) -> Result<Vec<Fill>> {
        let taker_hash = self.eip712.encode(ask);
        if let Some(existing_ask) = self.hash_to_order.get(&taker_hash) {
            if existing_ask.trader_address == ask.trader_address {
                return Err(Error::DuplicateOrder(taker_hash, ask.trader_address));
            }
        }

        // get possible fills
        let mut fills = vec![];
        for (_, bid) in self
            .bids
            .range((Unbounded, Included((Reverse(ask.price), ask.timestamp))))
        {
            if bid.trader_address == ask.trader_address {
                // self-match
                break;
            }

            // TODO: save so don't have to recompute
            let maker_hash = self.eip712.encode(*bid);
            let fill_amount = ask.amount.min(bid.amount);
            let fill = Fill {
                maker_hash,
                taker_hash,
                fill_amount,
                price: bid.price,
            };
            fills.push(fill);
            ask.amount -= fill_amount;
            if ask.amount == Decimal::ZERO {
                break;
            }
        }

        // update book to reflect fills
        for fill in &fills {
            let bid = self.hash_to_order[&fill.maker_hash];
            if bid.amount == fill.fill_amount {
                // fill completely uses up bid, remove
                self.bids.remove(&(Reverse(bid.price), bid.timestamp));
                self.hash_to_order.remove(&fill.maker_hash);
                *self.agg_bid_amt.get_mut(&Reverse(bid.price)).unwrap() -= ask.amount;
                if self.agg_bid_amt[&Reverse(bid.price)] == Decimal::ZERO {
                    self.agg_bid_amt.remove(&Reverse(bid.price));
                }
            } else {
                self.bids
                    .get_mut(&(Reverse(bid.price), bid.timestamp))
                    .unwrap()
                    .amount -= fill.fill_amount;
                *self.agg_bid_amt.get_mut(&Reverse(bid.price)).unwrap() -= ask.amount;
            }
        }

        if ask.amount > Decimal::ZERO {
            // add remaining ask to book
            self.asks.insert((ask.price, ask.timestamp), ask);
            self.hash_to_order
                .insert(taker_hash, self.asks[&(ask.price, ask.timestamp)]);
            *self.agg_ask_amt.entry(ask.price).or_insert(Decimal::ZERO) += ask.amount;
        }

        Ok(fills)
    }

    pub fn get_order(&self, order_hash: H256) -> Result<Order> {
        if let Some(order) = self.hash_to_order.get(&order_hash) {
            return Ok(*order);
        }
        Err(Error::OrderNotFound(order_hash))
    }

    pub fn delete_order(&mut self, order_hash: H256) -> Result<()> {
        if let Some(order) = self.hash_to_order.get(&order_hash) {
            match order.side {
                Side::Bid => {
                    self.bids.remove(&(Reverse(order.price), order.timestamp));
                    *self.agg_bid_amt.get_mut(&Reverse(order.price)).unwrap() -= order.amount;
                    if self.agg_bid_amt[&Reverse(order.price)] == Decimal::ZERO {
                        self.agg_bid_amt.remove(&Reverse(order.price));
                    }
                }
                Side::Ask => {
                    self.asks.remove(&(order.price, order.timestamp));
                    *self.agg_ask_amt.get_mut(&order.price).unwrap() -= order.amount;
                    if self.agg_ask_amt[&order.price] == Decimal::ZERO {
                        self.agg_ask_amt.remove(&order.price);
                    }
                }
            }
            self.hash_to_order.remove(&order_hash);
            return Ok(());
        }
        Err(Error::OrderNotFound(order_hash))
    }

    pub fn l2_snapshot(&self) -> L2OrderBook {
        let asks = self
            .agg_ask_amt
            .iter()
            .take(50)
            .map(|(price, amount)| L2Order {
                amount: *amount,
                price: *price,
            })
            .collect();
        let bids = self
            .agg_bid_amt
            .iter()
            .take(50)
            .map(|(price, amount)| L2Order {
                amount: *amount,
                price: price.0,
            })
            .collect();
        L2OrderBook { asks, bids }
    }
}
