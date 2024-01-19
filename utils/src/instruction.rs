use borsh::BorshSerialize;
use brc20_oracle::types::{Brc20Asset, Brc20Key, Brc20OracleInstruction, Committee};
use brc20_oracle::{ASSET_PREFIX, COMMITTEE_PREFIX};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};
use solana_sdk::ed25519_instruction::new_ed25519_instruction;
use solana_sdk::signer::{keypair::Keypair, Signer};

pub fn init_committee_ix(
    program_id: &Pubkey,
    payer: &Keypair,
    old_committee: Option<&Keypair>,
    new_committee: &Pubkey,
    id: u8,
) -> Vec<Instruction> {
    let (committee_info_address, _) = find_committee_address(program_id);
    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(committee_info_address.clone(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
    ];

    let new_committee = Committee {
        id,
        address: *new_committee,
        uid: 0,
    };
    let sign_msg = new_committee.try_to_vec().unwrap();

    let signer = old_committee.unwrap_or(payer);
    let verify_instruction = new_ed25519_instruction(
        &ed25519_dalek::Keypair::from_bytes(&signer.to_bytes()).unwrap(),
        &sign_msg,
    );
    let signature = signer.sign_message(&sign_msg).as_ref().to_vec();
    let data = Brc20OracleInstruction::SetCommittee(new_committee, signature)
        .try_to_vec()
        .unwrap();
    vec![
        verify_instruction,
        Instruction {
            program_id: program_id.clone(),
            accounts,
            data,
        },
    ]
}

pub fn request_ix(program_id: &Pubkey, payer: &Keypair, key: Brc20Key) -> Vec<Instruction> {
    let (committee_info_address, _) = find_committee_address(program_id);
    let (asset_address, _) = find_asset_address(program_id, &key);
    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(committee_info_address, false),
        AccountMeta::new(asset_address, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let data = Brc20OracleInstruction::Request(key).try_to_vec().unwrap();
    vec![
        Instruction {
            program_id: program_id.clone(),
            accounts,
            data,
        }
    ]
}

pub fn insert_ix(
    program_id: &Pubkey,
    committee: &Keypair,
    committee_info: Pubkey,
    uid: u64,
    key: Brc20Key,
    amount: u128,
) -> Vec<Instruction> {
    let (asset_address, _) = find_asset_address(program_id, &key);

    let accounts = vec![
        AccountMeta::new_readonly(committee_info.clone(), false),
        AccountMeta::new(asset_address, false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
    ];
    let asset = Brc20Asset {
        prefix: ASSET_PREFIX,
        uid,
        set: true,
        key: key.clone(),
        amount,
    };
    let asset_msg = asset.try_to_vec().unwrap();
    let signature = committee.sign_message(&asset_msg).as_ref().to_vec();
    let data = Brc20OracleInstruction::Insert(key, amount, signature)
        .try_to_vec()
        .unwrap();

    let verify_instruction = new_ed25519_instruction(
        &ed25519_dalek::Keypair::from_bytes(&committee.to_bytes()).unwrap(),
        &asset_msg,
    );
    vec![
        verify_instruction,
        Instruction {
            program_id: program_id.clone(),
            accounts,
            data,
        },
    ]
}

pub fn find_committee_address(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&COMMITTEE_PREFIX], program_id)
}

pub fn find_asset_address(program_id: &Pubkey, key: &Brc20Key) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            &ASSET_PREFIX,
            solana_program::keccak::hash(key.try_to_vec().unwrap().as_slice()).as_ref(),
        ],
        program_id,
    )
}
