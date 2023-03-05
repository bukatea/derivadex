mod orderbook;
use orderbook::{L2OrderBook, OrderBook};

mod error;
pub use error::EngineError;
use error::{EngineError as Error, Result};

use std::collections::HashMap;
use web3::types::{Address, H256};

use crate::{Account, Fill, Order, Side};

pub struct Engine {
    accounts: HashMap<Address, Account>,
    // order hash to trader address, for updating balances
    hash_to_address: HashMap<H256, Address>,
    book: OrderBook,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            hash_to_address: HashMap::new(),
            book: OrderBook::new(),
        }
    }

    pub fn create_account(&mut self, mut account: Account) -> Result<Address> {
        if self.accounts.contains_key(&account.trader_address) {
            return Err(Error::AccountAlreadyExists(account.trader_address));
        }
        // validate balances
        if account.ddx_balance.is_sign_negative() {
            return Err(Error::NegativeBalance(account.ddx_balance));
        }
        if account.usd_balance.is_sign_negative() {
            return Err(Error::NegativeBalance(account.usd_balance));
        }
        // rescale to 18 decimal places
        account.usd_balance.rescale(18);
        account.ddx_balance.rescale(18);
        Ok(account.trader_address)
    }

    pub fn get_account(&self, address: Address) -> Result<Account> {
        if let Some(account) = self.accounts.get(&address) {
            return Ok(*account);
        }
        Err(Error::AccountNotFound(address))
    }

    pub fn delete_account(&mut self, address: Address) -> Result<()> {
        if let Some(_) = self.accounts.remove(&address) {
            return Ok(());
        }

        Err(Error::AccountNotFound(address))
    }

    pub fn create_order(&mut self, order: Order) -> Result<Vec<Fill>> {
        let taker = self.accounts[&order.trader_address];
        match order.side {
            Side::Bid => {
                // check if enough usd balance
                let usd_cost = order.amount * order.price;
                if taker.usd_balance - taker.usd_book_outstanding < usd_cost {
                    return Err(Error::InsufficientBalance(taker.usd_balance, usd_cost));
                }
                // update account
                self.accounts
                    .get_mut(&order.trader_address)
                    .unwrap()
                    .usd_book_outstanding += usd_cost;
                self.book
                    .add_bid(order)
                    .map(|fills| {
                        fills.iter().for_each(|fill| {
                            let taker = self.accounts.get_mut(&order.trader_address).unwrap();
                            let usd_cost = fill.fill_amount * fill.price;
                            taker.usd_balance -= usd_cost;
                            taker.usd_book_outstanding -= usd_cost;
                            let maker = self
                                .accounts
                                .get_mut(&self.hash_to_address[&fill.maker_hash])
                                .unwrap();
                            maker.ddx_balance -= fill.fill_amount;
                            maker.ddx_book_outstanding -= fill.fill_amount;
                        });
                        fills
                    })
                    .map_err(|e| e.into())
            }
            Side::Ask => {
                // check if enough ddx balance
                let ddx_cost = order.amount;
                if taker.ddx_balance - taker.ddx_book_outstanding < ddx_cost {
                    return Err(Error::InsufficientBalance(taker.ddx_balance, ddx_cost));
                }
                // update account
                self.accounts
                    .get_mut(&order.trader_address)
                    .unwrap()
                    .ddx_book_outstanding += ddx_cost;
                self.book
                    .add_ask(order)
                    .map(|fills| {
                        fills.iter().for_each(|fill| {
                            let taker = self.accounts.get_mut(&order.trader_address).unwrap();
                            let usd_cost = fill.fill_amount * fill.price;
                            taker.ddx_balance -= fill.fill_amount;
                            taker.ddx_book_outstanding -= fill.fill_amount;
                            let maker = self
                                .accounts
                                .get_mut(&self.hash_to_address[&fill.maker_hash])
                                .unwrap();
                            maker.ddx_balance -= usd_cost;
                            maker.ddx_book_outstanding -= usd_cost;
                        });
                        fills
                    })
                    .map_err(|e| e.into())
            }
        }
    }

    pub fn get_order(&self, order_hash: H256) -> Result<Order> {
        self.book.get_order(order_hash).map_err(|e| e.into())
    }

    pub fn delete_order(&mut self, order_hash: H256) -> Result<()> {
        self.book.delete_order(order_hash).map_err(|e| e.into())
    }

    pub fn get_book(&self) -> L2OrderBook {
        self.book.l2_snapshot()
    }
}
