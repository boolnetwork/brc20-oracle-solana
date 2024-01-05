use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub enum Brc20OracleInstruction {
    SetCommittee(Pubkey),
    Request(Brc20Key),
    Insert(Brc20Key, u128),
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
pub struct Brc20Key {
    pub height: u32,
    pub tick: [u8; 4],
    pub owner: String,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
pub struct Brc20Asset {
    pub key: Brc20Key,
    pub amount: u128,
}
