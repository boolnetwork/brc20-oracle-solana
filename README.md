## Types

```Rust
pub struct Pubkey(pub(crate) [u8; 32]);

pub enum Brc20OracleInstruction {
    SetCommittee(Pubkey),
    Request(Brc20Key),
    Insert(Brc20Key, u128),
}

pub struct Brc20Key {
    pub height: u32,
    pub tick: [u8; 4],
    pub owner: String,
}

pub struct Brc20Asset {
    pub key: Brc20Key,
    pub amount: u128,
}
```

## Consts
```Rust
const COMMITTEE_PREFIX: &[u8] = b"Brc20OracleCommittee";
const ASSET_PREFIX: &[u8] = b"Brc20OracleAsset";
```

## Storages
### *Committee*:

"Description": admin account who can insert real [Brc20Asset] data.

"AddressDerivation": `Pubkey::find_program_address(&[COMMITTEE_PREFIX], &program_id);`

"DataType": `Pubkey` from solana definition.

### *Brc20Asset*:

"Description": Actual data for specific brc20 asset.

"AddressDerivation": key is struct [Brc20Key]. `Pubkey::find_program_address(&[ASSET_PREFIX, key.try_to_vec()?.as_slice()], program_id);`

"DataType": [Brc20Assset].


## Local 
test-node: `solana-test-validator`

build library: `cargo build-bpf`

run `cargo update -p ahash@0.8.7 --precise 0.8.6` if error.

deploy: `solana program deploy ./target/deploy/hello_world.so`

## Test
`cargo test`
