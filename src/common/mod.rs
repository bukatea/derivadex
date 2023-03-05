use rust_decimal::Decimal;
use serde::{de::Visitor, Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use web3::types::{Address, H256, U256};

#[derive(Debug, Copy, Clone, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub ddx_balance: Decimal,
    pub usd_balance: Decimal,
    pub trader_address: Address,

    #[serde(skip)]
    pub ddx_book_outstanding: Decimal,
    #[serde(skip)]
    pub usd_book_outstanding: Decimal,
}

#[derive(Debug, Copy, Clone, Serialize)]
pub struct Nonce(pub H256);

impl<'de> Deserialize<'de> for Nonce {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NonceVisitor;

        impl<'de> Visitor<'de> for NonceVisitor {
            type Value = Nonce;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a number or decimal string")
            }

            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Nonce(H256::from_low_u64_be(v as u64)))
            }

            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Nonce(H256::from_low_u64_be(v as u64)))
            }

            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Nonce(H256::from_low_u64_be(v as u64)))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Nonce(H256::from_low_u64_be(v)))
            }

            fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let mut bytes = [0u8; 32];
                U256::from(v).to_big_endian(&mut bytes);
                Ok(Nonce(H256::from_slice(&bytes)))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let mut bytes = [0u8; 32];
                U256::from_dec_str(v)
                    .map_err(|e| serde::de::Error::custom(e))?
                    .to_big_endian(&mut bytes);
                Ok(Nonce(H256::from_slice(&bytes)))
            }
        }

        deserializer.deserialize_any(NonceVisitor)
    }
}

impl std::ops::Deref for Nonce {
    type Target = H256;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub amount: Decimal,
    pub nonce: Nonce,
    pub price: Decimal,
    pub side: Side,
    pub trader_address: Address,

    #[serde(skip)]
    pub timestamp: u128,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fill {
    pub maker_hash: H256,
    pub taker_hash: H256,
    pub fill_amount: Decimal,
    pub price: Decimal,
}
