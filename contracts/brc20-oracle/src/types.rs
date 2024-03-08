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

#[test]
fn test_parse_asset() {
    use crate::ASSET_PREFIX; // "Asset"

    let asset = Brc20Asset {
        prefix: ASSET_PREFIX,
        set: true,
        uid: 12,
        key: Brc20Key {
            height: 1564854,
            tick: *b"ordi",
            address: BtcAddress { network: 1, address_type: AddressType::P2wpkh([8u8; 33]) },
        },
        amount: 1000000,
    };

    // 链上获取的账号对应的data.
    let data: Vec<u8> = asset.try_to_vec().unwrap();

    // 手动解析整个数据
    // 1. prefix: 固定占用前5个字节且固定为“Asset”
    let mut prefix = [0u8; 5];
    prefix.copy_from_slice(&data[0..5]);

    // 2. set: 类型为bool, 固定占用1个字节
    let set = match data[5] {
        0 => false,
        1 => true,
        _ => unreachable!(),
    };

    // 3. uid: 类型为u64, 固定占用8个字节
    let mut uid_u64_le_bytes = [0u8; 8];
    uid_u64_le_bytes.copy_from_slice(&data[6..14]);
    let uid = u64::from_le_bytes(uid_u64_le_bytes);

    // 4. key.height: 类型为u32, 固定占用4个字节
    let mut height_u32_le_bytes = [0u8; 4];
    height_u32_le_bytes.copy_from_slice(&data[14..18]);
    let height = u32::from_le_bytes(height_u32_le_bytes);
    println!("height: {height}");

    // 5. key.tick: 固定占用4个字节
    let mut tick = [0u8; 4];
    tick.copy_from_slice(&data[18..22]);
    println!("tick: {}", String::from_utf8_lossy(&tick));

    // 6. key.address.network: 类型为u8, 固定占用1个字节
    // 有4种取值0/1/2/3分别对应Bitcoin/Testnet/Signet/Regtest
    let network = data[22];
    println!("network: {network}");

    // 7. key.address.address_type
    // 先获取1个字节为枚举的编号0, 1, 2, 3分别对应四种公钥类型:
    // P2pkh P2wpkh P2trUnTweaked P2trTweaked
    // 后续根据类型获取对应长度的公钥, 分别对应33, 33, 64, 32
    let address_type_num = data[23];
    let mut offset = 24usize;
    let address_type = match address_type_num {
        0 => {
            offset += 33;
            let mut pk = [0u8; 33];
            pk.copy_from_slice(&data[24..offset]);
            AddressType::P2pkh(pk)
        },
        1 => {
            offset += 33;
            let mut pk = [0u8; 33];
            pk.copy_from_slice(&data[24..offset]);
            AddressType::P2wpkh(pk)
        },
        2 => {
            offset += 64;
            let mut pk = [0u8; 64];
            pk.copy_from_slice(&data[24..offset]);
            AddressType::P2trUnTweaked(pk)
        },
        3 => {
            offset += 32;
            let mut pk = [0u8; 32];
            pk.copy_from_slice(&data[24..offset]);
            AddressType::P2trTweaked(pk)
        },
        _ => unreachable!()
    };
    println!("address_type: {address_type:?}");

    // 8. amount: 类型为u128, 占据最后16个字节
    let mut amount_u128_le_bytes = [0u8; 16];
    amount_u128_le_bytes.copy_from_slice(&data[offset..]);
    let amount = u128::from_le_bytes(amount_u128_le_bytes);
    println!("amount: {amount}");

    // 重构结构体
    let parsed_asset = Brc20Asset {
        prefix,
        set,
        uid,
        key: Brc20Key {
            height,
            tick,
            address: BtcAddress { network, address_type },
        },
        amount,
    };
    // 判断是否正确
    assert_eq!(asset, parsed_asset);
}
