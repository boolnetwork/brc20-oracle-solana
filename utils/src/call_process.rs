use solana_program::pubkey::Pubkey;
use solana_program::instruction::Instruction;
use solana_sdk::signer::keypair::Keypair;
use solana_client::client_error::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use solana_sdk::signers::Signers;
use brc20_oracle::types::Brc20Key;
use crate::instruction::*;

pub async fn call_init_committee(
    url: &str,
    commitment: CommitmentConfig,
    program_id: &Pubkey,
    payer: &Keypair,
    old_committee: Option<&Keypair>,
    new_committee: &Pubkey,
    id: u8,
) -> Result<Signature> {
    let client = RpcClient::new_with_commitment(url.to_string(), commitment);
    let ixs = init_committee_ix(program_id, payer, old_committee, new_committee, id);
    process_instruction(&client, payer, &[payer], &ixs).await
}

pub async fn call_request(
    url: &str,
    commitment: CommitmentConfig,
    program_id: &Pubkey,
    payer: &Keypair,
    key: &Brc20Key,
) -> Result<Signature> {
    let client = RpcClient::new_with_commitment(url.to_string(), commitment);
    let ixs = request_ix(program_id, payer, key.clone());
    process_instruction(&client, payer, &[payer], &ixs).await
}

pub async fn call_insert(
    url: &str,
    commitment: CommitmentConfig,
    payer: &Keypair,
    program_id: &Pubkey,
    committee: &Keypair,
    uid: u64,
    key: Brc20Key,
    amount: u128,
) -> Result<Signature> {
    let client = RpcClient::new_with_commitment(url.to_string(), commitment);
    let committee_info = find_committee_address(program_id).0;
    let ixs = insert_ix(program_id, committee, committee_info, uid, key, amount);
    process_instruction(&client, payer, &[payer], &ixs).await
}

pub async fn process_instruction<T: Signers>(
    client: &RpcClient,
    payer: &Keypair,
    signers: &T,
    instructions: &[Instruction],
) -> Result<Signature> {
    let mut transaction = Transaction::new_with_payer(instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_latest_blockhash().await?;
    transaction.sign(signers, recent_blockhash);
    client.send_and_confirm_transaction(&transaction).await
}

#[cfg(test)]
pub mod call_tests {
    use solana_program_test::tokio;
    use borsh::BorshDeserialize;
    use brc20_oracle::types::{BtcAddress, Network};
    use crate::call_process::*;

    #[tokio::test]
    pub async fn test_init_committee() {
        let url = "https://api.devnet.solana.com";
        let program_id = Pubkey::try_from("6Z69Yzja3ZUHs6WrZxNMs823nUc3bEZDMkfjbkqUHKZY").unwrap();
        let payer_sk = [];
        let payer = Keypair::from_bytes(&payer_sk).unwrap();
        let committee_pk = hex::decode("02f48c4bda350e728d9952dc209323a7ac2f0a1ffe56f342e40c88eeb90892f7").unwrap();
        let committee = Pubkey::try_from_slice(&committee_pk).unwrap();

        let signature = call_init_committee(url, CommitmentConfig::confirmed(), &program_id, &payer, None, &committee, 0).await.unwrap();
    }

    #[tokio::test]
    pub async fn test_request() {
        let url = "https://api.devnet.solana.com";
        let program_id = Pubkey::try_from("6Z69Yzja3ZUHs6WrZxNMs823nUc3bEZDMkfjbkqUHKZY").unwrap();
        let payer_sk = [];
        let payer = Keypair::from_bytes(&payer_sk).unwrap();

        let btc_compressed_pk = hex::decode("02a9ae12a3aed9a046167a3e9e6a408d13e8b8ab4c02df55a35d7db1eb636610e6").unwrap();

        // [2575747, 2575855]
        for height in 2575773..2575774 {
            let key = Brc20Key {
                height,
                tick: *b"sats",
                address: BtcAddress::p2wpkh(Network::Testnet.to_u8(), &btc_compressed_pk).unwrap(),
            };

            let signature = call_request(url, CommitmentConfig::confirmed(), &program_id, &payer, &key).await.unwrap();
            println!("send height: {height}, with sig: {}", signature);
        }
    }
}

