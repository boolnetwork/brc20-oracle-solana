use std::str::FromStr;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program_test::*;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::{system_instruction, system_program, sysvar};
use solana_sdk::ed25519_instruction::new_ed25519_instruction;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::signers::Signers;
use solana_sdk::transaction::Transaction;
use crate::types::{Brc20Asset, Brc20Key, Brc20OracleInstruction, Committee};
use crate::{COMMITTEE_PREFIX, ASSET_PREFIX};

const PROGRAM_ID: &str = "1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM";

pub async fn process<T: Signers>(
    client: &mut BanksClient,
    payer: &Keypair,
    signers: &T,
    instructions: &[Instruction],
) -> Result<(), BanksClientError> {
    let mut transaction = Transaction::new_with_payer(instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_latest_blockhash().await?;
    transaction.sign(signers, recent_blockhash);

    client.process_transaction(transaction).await
}

pub async fn query_data<T: BorshDeserialize>(
    banks_client: &mut BanksClient,
    account_id: Pubkey,
) -> T {
    let account = banks_client.get_account(account_id).await.unwrap().unwrap();
    T::try_from_slice(&account.data).unwrap()
}

pub async fn init_client() -> (BanksClient, Keypair) {
    let mut program_test = ProgramTest::default();
    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();
    // load programs
    program_test.add_program(
        "brc20_oracle",
        program_id,
        processor!(crate::process_instruction),
    );
    let (banks_client, payer, _) = program_test.start().await;
    (banks_client, payer)
}

pub async fn process_init_committee(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    old_committee: &Keypair,
    new_committee: &Pubkey,
    id: u8,
) -> Pubkey {
    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();

    let (committee_info_address, _) =
        Pubkey::find_program_address(&[&COMMITTEE_PREFIX], &program_id);

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(committee_info_address.clone(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
    ];

    let new_committee = Committee { id, address: *new_committee };
    let sign_msg = new_committee.try_to_vec().unwrap();

    let verify_instruction = new_ed25519_instruction(
        &ed25519_dalek::Keypair::from_bytes(&old_committee.to_bytes()).unwrap(),
        &sign_msg,
    );
    let signature = old_committee.sign_message(&sign_msg).as_ref().to_vec();
    let data = Brc20OracleInstruction::SetCommittee(new_committee, signature).try_to_vec().unwrap();
    let instruction = Instruction {
        program_id,
        accounts,
        data,
    };
    process(banks_client, payer, &[payer], &[verify_instruction, instruction]).await.unwrap();
    committee_info_address
}

pub async fn process_query(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    key: Brc20Key,
) -> Pubkey {
    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();

    let (asset_address, _) =
        Pubkey::find_program_address(&[&ASSET_PREFIX, key.try_to_vec().unwrap().as_slice()], &program_id);
    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(asset_address, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let data = Brc20OracleInstruction::Request(key).try_to_vec().unwrap();
    let instruction = Instruction {
        program_id,
        accounts,
        data,
    };
    process(banks_client, payer, &[payer], &[instruction]).await.unwrap();
    asset_address
}

pub async fn process_insert(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    committee: &Keypair,
    committee_info: Pubkey,
    key: Brc20Key,
    amount: u128,
) -> Pubkey {
    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();

    let (asset_address, _) =
        Pubkey::find_program_address(&[&ASSET_PREFIX, key.try_to_vec().unwrap().as_slice()], &program_id);

    let accounts = vec![
        AccountMeta::new_readonly(committee_info.clone(), false),
        AccountMeta::new(asset_address, false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
    ];
    let brc20_asset = Brc20Asset { prefix: ASSET_PREFIX, key: key.clone(), amount };
    let asset_msg = brc20_asset.try_to_vec().unwrap();
    let signature = committee.sign_message(&asset_msg).as_ref().to_vec();
    let data = Brc20OracleInstruction::Insert(key, amount, signature).try_to_vec().unwrap();

    let verify_instruction = new_ed25519_instruction(
        &ed25519_dalek::Keypair::from_bytes(&committee.to_bytes()).unwrap(),
        &asset_msg,
    );
    let instruction = Instruction {
        program_id,
        accounts,
        data,
    };
    process(banks_client, payer, &[payer], &[verify_instruction, instruction]).await.unwrap();
    asset_address
}

#[tokio::test]
async fn test_brc20_oracle() {
    let (mut banks_client, payer) = init_client().await;
    println!("payer: {:?}", payer.pubkey());
    let init_committee_pair = Keypair::new();
    let new_committee_pair = Keypair::new();

    // initialize committee
    let committee_info_address = process_init_committee(&mut banks_client, &payer, &init_committee_pair, &init_committee_pair.pubkey(), 0).await;
    let committee: Committee = query_data(&mut banks_client, committee_info_address).await;
    assert_eq!(committee.id, 0);
    assert_eq!(committee.address, init_committee_pair.pubkey());

    // change committee
    let committee_info_address = process_init_committee(&mut banks_client, &payer, &init_committee_pair, &new_committee_pair.pubkey(), 1).await;
    let committee: Committee = query_data(&mut banks_client, committee_info_address).await;
    assert_eq!(committee.id, 1);
    assert_eq!(committee.address, new_committee_pair.pubkey());

    // query brc20 amount
    let key = Brc20Key { height: 1, tick: [1, 2, 3, 4], owner: "12345".to_string() };
    let asset_address = process_query(&mut banks_client, &payer, key.clone()).await;
    let asset: Brc20Asset = query_data(&mut banks_client, asset_address).await;
    assert_eq!(key, asset.key);
    assert_eq!(0, asset.amount);

    // insert brc20 amount
    let mut amount = 1000;
    let asset_address = process_insert(
        &mut banks_client,
        &payer,
        &new_committee_pair,
        committee_info_address,
        key.clone(),
        amount
    ).await;
    let asset: Brc20Asset = query_data(&mut banks_client, asset_address).await;
    assert_eq!(amount, asset.amount);

    // update brc20 amount
    amount = 2000;
    let asset_address = process_insert(
        &mut banks_client,
        &payer,
        &new_committee_pair,
        committee_info_address,
        key,
        amount
    ).await;
    let asset: Brc20Asset = query_data(&mut banks_client, asset_address).await;
    assert_eq!(amount, asset.amount);
}
