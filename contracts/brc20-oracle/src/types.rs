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
    // Authority id to prevent duplicate submit for authority change.
    pub id: u8,
    pub address: Pubkey,
    // Counter for requests(assets)
    pub uid: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Copy, PartialEq, Eq, Clone, Debug)]
pub enum Network {
    /// Classic Bitcoin
    Bitcoin,
    /// Bitcoin's testnet
    Testnet,
    /// Bitcoin's signet
    Signet,
    /// Bitcoin's regtest
    Regtest,
}

impl Network {
    pub fn from(network: u8) -> Result<Self, u8> {
        Ok(match network {
            0 => Self::Bitcoin,
            1 => Self::Testnet,
            2 => Self::Signet,
            3 => Self::Regtest,
            _ => return Err(network),
        })
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Bitcoin => 0,
            Self::Testnet => 1,
            Self::Signet => 2,
            Self::Regtest => 3,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
pub enum AddressType {
    // compressed pk
    P2pkh([u8; 33]),
    // compressed pk
    P2wpkh([u8; 33]),
    // 32 bytes for x_only_public_key, 32 bytes for tap_tweak_hash([0u8; 32] means no tap_tweak_hash)
    P2trUnTweaked([u8; 64]),
    // tweaked x_only_public_key
    P2trTweaked([u8; 32]),
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
pub struct BtcAddress {
    pub network: u8,
    pub address_type: AddressType,
}

impl BtcAddress {
    pub fn to_bump_seed(&self) -> Vec<u8> {
        solana_program::keccak::hash(&self.try_to_vec().unwrap()).as_ref().to_vec()
    }

    pub fn p2pkh(network: u8, compressed_pk: &[u8]) -> Result<Self, String> {
        if compressed_pk.len() != 33 {
            return Err("invalid p2pkh compressed_pk length".to_string());
        }
        let mut compressed = [0u8; 33];
        compressed.copy_from_slice(compressed_pk);
        Ok(BtcAddress { network, address_type: AddressType::P2pkh(compressed) })
    }

    pub fn p2wpkh(network: u8, compressed_pk: &[u8]) -> Result<Self, String> {
        if compressed_pk.len() != 33 {
            return Err("invalid p2wpkh compressed_pk length".to_string());
        }
        let mut compressed = [0u8; 33];
        compressed.copy_from_slice(compressed_pk);
        Ok(BtcAddress { network, address_type: AddressType::P2wpkh(compressed) })
    }

    pub fn p2tr_untweaked(network: u8, internal_key: &[u8], tap_tweak_hash: Option<&[u8]>) -> Result<Self, String> {
        if internal_key.len() != 32 {
            return Err("invalid internal_key length".to_string());
        }
        let mut untweaked = [0u8; 64];
        untweaked[..32].copy_from_slice(internal_key);
        if let Some(hash) = tap_tweak_hash {
            if hash.len() != 32 {
                return Err("invalid tap_tweak_hash length".to_string());
            }
            untweaked[32..].copy_from_slice(hash);
        }

        Ok(BtcAddress { network, address_type: AddressType::P2trUnTweaked(untweaked) })
    }

    pub fn p2tr_tweaked(network: u8, tweaked_pk: &[u8]) -> Result<Self, String> {
        if tweaked_pk.len() != 33 {
            return Err("invalid tweaked_pk length".to_string());
        }
        let mut tweaked = [0u8; 32];
        tweaked.copy_from_slice(tweaked_pk);
        Ok(BtcAddress { network, address_type: AddressType::P2trTweaked(tweaked) })
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
pub struct Brc20Key {
    pub height: u32,
    pub tick: [u8; 4],
    pub address: BtcAddress,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
pub struct Brc20Asset {
    // To filter this account easily by client, we set same prefix.
    pub prefix: [u8; 5],
    // if the asset is set.
    pub set: bool,
    pub uid: u64,
    pub key: Brc20Key,
    pub amount: u128,
}
