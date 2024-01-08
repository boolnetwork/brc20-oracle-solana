use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub enum Brc20OracleInstruction {
    SetCommittee(Committee, Vec<u8>),
    Request(Brc20Key),
    Insert(Brc20Key, u128, Vec<u8>),
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
pub struct Committee {
    pub id: u8,
    pub address: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
pub struct Brc20Key {
    pub height: u32,
    pub tick: [u8; 4],
    pub owner: String,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
pub struct Brc20Asset {
    // To filter this account easily by client, we set same prefix.
    pub prefix: [u8; 5],
    pub key: Brc20Key,
    pub amount: u128,
}
