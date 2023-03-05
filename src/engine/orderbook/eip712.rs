use lazy_static::lazy_static;
use web3::{
    signing::keccak256,
    types::{Address, H256, U256},
};

lazy_static! {
    static ref DOMAIN_HASH: [u8; 32] =
        keccak256("EIP712Domain(string name,string version)".as_bytes());
}

// time permitted, I would have added macros to auto implement
// TypeHashable and EncodeDataable for a struct
pub trait TypeHashable {
    fn type_hash(&self) -> [u8; 32];
}

pub trait EncodeDataable {
    fn encode_data(&self) -> Vec<u8>;
}

impl EncodeDataable for U256 {
    fn encode_data(&self) -> Vec<u8> {
        let mut arr = [0; 32];
        self.to_big_endian(&mut arr);
        arr.to_vec()
    }
}

impl EncodeDataable for u8 {
    fn encode_data(&self) -> Vec<u8> {
        U256::from(*self).encode_data()
    }
}

impl EncodeDataable for Address {
    fn encode_data(&self) -> Vec<u8> {
        U256::from(self.as_bytes()).encode_data()
    }
}

impl EncodeDataable for &'static str {
    fn encode_data(&self) -> Vec<u8> {
        keccak256(self.as_bytes()).to_vec()
    }
}

pub trait HashStructable: TypeHashable + EncodeDataable {
    fn hash_struct(&self) -> [u8; 32] {
        keccak256(&[self.type_hash().as_ref(), &self.encode_data()].concat())
    }
}

impl<T: TypeHashable + EncodeDataable> HashStructable for T {}

pub struct Eip712Domain {
    pub name: &'static str,
    pub version: &'static str,
}

impl TypeHashable for Eip712Domain {
    fn type_hash(&self) -> [u8; 32] {
        *DOMAIN_HASH
    }
}

impl EncodeDataable for Eip712Domain {
    fn encode_data(&self) -> Vec<u8> {
        [self.name.encode_data(), self.version.encode_data()].concat()
    }
}

pub struct Eip712 {
    pub domain: Eip712Domain,
}

impl Eip712 {
    pub fn new(domain: Eip712Domain) -> Self {
        Self { domain }
    }

    pub fn encode(&self, message: impl HashStructable) -> H256 {
        keccak256(
            &[
                [0x19u8, 0x01u8].as_ref(),
                &self.domain.hash_struct(),
                &message.hash_struct(),
            ]
            .concat(),
        )
        .into()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use web3::ethabi::ethereum_types::BigEndianHash;
    use web3::ethabi::Uint;

    use super::*;
    use crate::{Order, Side};

    #[test]
    fn test_eip712() {
        let eip712 = Eip712::new(Eip712Domain {
            name: "DDX take-home",
            version: "0.1.0",
        });
        let order = Order {
            amount: 1234.into(),
            nonce: H256::from_uint(&Uint::from_dec_str("12").unwrap()),
            price: 5432.into(),
            side: Side::Bid,
            trader_address: Address::from_str("0x3A880652F47bFaa771908C07Dd8673A787dAEd3A")
                .unwrap(),
            timestamp: 0,
        };
        let hash = eip712.encode(order);
        assert_eq!(
            hash,
            H256::from_str("0x15a7b83cc86b50aaa2fa0c0871d5dbaae62f116436291e976c84b034b58cb728")
                .unwrap()
        );
    }
}
