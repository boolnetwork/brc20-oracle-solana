pub mod types;
pub mod error;
#[cfg(test)]
mod test;

use borsh::{BorshSerialize, BorshDeserialize};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey, system_instruction};
use solana_program::account_info::next_account_info;
use solana_program::program::invoke_signed;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::ed25519_program::ID as ED25519_ID;
use solana_program::instruction::Instruction;
use solana_program::sysvar::instructions::load_instruction_at_checked;
use types::*;
use error::Brc20OracleError;

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

const COMMITTEE_PREFIX: [u8; 9] = *b"Committee";
const ASSET_PREFIX: [u8; 5] = *b"Asset";

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = Brc20OracleInstruction::try_from_slice(instruction_data)?;
    match instruction {
        Brc20OracleInstruction::SetCommittee(committee, signature) => set_committee(program_id, accounts, committee, signature),
        Brc20OracleInstruction::Request(key) => request(program_id, accounts, key),
        Brc20OracleInstruction::Insert(key, amount, signature) => insert(program_id, accounts, key, amount, signature),
    }
}

pub fn set_committee(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    committee: Committee,
    signature: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer_info = next_account_info(account_info_iter)?;
    let committee_info = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let ix_sysvar_info = next_account_info(account_info_iter)?;

    let (committee_address, bump) = Pubkey::find_program_address(
        &[&COMMITTEE_PREFIX],
        program_id,
    );
    if committee_info.key != &committee_address {
        return Err(Brc20OracleError::IncorrectCommitteePDA.into());
    }

    let parse_committee = Committee::try_from_slice(&committee_info.data.borrow());
    match parse_committee {
        Ok(brc20_committee) => {
            if committee_info.owner != program_id {
                return Err(Brc20OracleError::NotOwnedByBrc20Oracle.into());
            }
            if committee.id != brc20_committee.id + 1 {
                return Err(Brc20OracleError::IncorrectCommitteeId.into());
            }
            let ix: Instruction = load_instruction_at_checked(0, ix_sysvar_info)?;
            verify_ed25519_ix(&ix, brc20_committee.address.as_ref(), &committee.try_to_vec()?, &signature)?;
        }
        Err(_) => {
            if committee.id != 0 {
                return Err(Brc20OracleError::IncorrectCommitteeId.into());
            }
            let size = committee.try_to_vec()?.len();
            invoke_signed(
                &system_instruction::create_account(
                    payer_info.key,
                    &committee_info.key,
                    Rent::get()?.minimum_balance(size),
                    size as u64,
                    program_id,
                ),
                &[payer_info.clone(), committee_info.clone(), system_program.clone()],
                &[&[&COMMITTEE_PREFIX, &[bump]]],
            )?;
        }
    }
    committee.serialize(&mut &mut committee_info.data.borrow_mut()[..])?;
    msg!("set committee: {:?}", committee);
    Ok(())
}

pub fn request(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    key: Brc20Key,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer_info = next_account_info(account_info_iter)?;
    let brc20_asset_info = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // initialize corresponding asset account rents.
    let (asset_address, bump) = Pubkey::find_program_address(
        &[&ASSET_PREFIX, key.try_to_vec()?.as_slice()],
        program_id,
    );
    if &asset_address != brc20_asset_info.key {
        return Err(Brc20OracleError::IncorrectAssetPDA.into());
    }
    let parse_amount = Brc20Asset::try_from_slice(&brc20_asset_info.data.borrow());
    match parse_amount {
        Ok(_) => return Err(Brc20OracleError::DuplicateRequest.into()),
        Err(_) => {
            let asset = Brc20Asset { prefix: ASSET_PREFIX, key: key.clone(), amount: 0 };
            let size = asset.try_to_vec()?.len();
            invoke_signed(
                &system_instruction::create_account(
                    payer_info.key,
                    brc20_asset_info.key,
                    Rent::get()?.minimum_balance(size),
                    size as u64,
                    program_id,
                ),
                &[payer_info.clone(), brc20_asset_info.clone(), system_program.clone()],
                &[&[&ASSET_PREFIX, key.try_to_vec()?.as_slice(), &[bump]]],
            )?;
            asset.serialize(&mut &mut brc20_asset_info.data.borrow_mut()[..])?;
            msg!("new request for key: {:?}", key);
        }
    }
    Ok(())
}

pub fn insert(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    key: Brc20Key,
    amount: u128,
    signature: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let committee_info = next_account_info(account_info_iter)?;
    let brc20_asset_info = next_account_info(account_info_iter)?;
    let ix_sysvar_info = next_account_info(account_info_iter)?;

    // check committee info's correctness.
    if committee_info.owner != program_id {
        return Err(Brc20OracleError::NotOwnedByBrc20Oracle.into());
    }
    let (committee_info_address, _) = Pubkey::find_program_address(&[&COMMITTEE_PREFIX], program_id);
    if &committee_info_address != committee_info.key {
        return Err(Brc20OracleError::IncorrectCommitteePDA.into());
    }
    // check corresponding amount address's correctness.
    let (asset_address, _) = Pubkey::find_program_address(
        &[&ASSET_PREFIX, key.try_to_vec()?.as_slice()],
        program_id,
    );
    if &asset_address != brc20_asset_info.key {
        return Err(Brc20OracleError::IncorrectAssetPDA.into());
    }
    // check corresponding amount's owner.
    if brc20_asset_info.owner != program_id {
        return Err(Brc20OracleError::NotOwnedByBrc20Oracle.into());
    }

    // check committee's signature.
    let committee = Committee::try_from_slice(&committee_info.data.borrow())?;
    let new_asset = Brc20Asset { prefix: ASSET_PREFIX, key, amount };
    let ix: Instruction = load_instruction_at_checked(0, ix_sysvar_info)?;
    verify_ed25519_ix(&ix, committee.address.as_ref(), &new_asset.try_to_vec()?, &signature)?;

    // update if initialized.
    let asset = Brc20Asset::try_from_slice(&brc20_asset_info.data.borrow());
    match asset {
        Ok(_) => {
            new_asset.serialize(&mut &mut brc20_asset_info.data.borrow_mut()[..])?;
            msg!("update asset: {:?}", new_asset);
        },
        Err(_) => return Err(Brc20OracleError::RequestNotInitialized.into())
    }

    Ok(())
}

pub fn verify_ed25519_ix(ix: &Instruction, pubkey: &[u8], msg: &[u8], sig: &[u8]) -> ProgramResult {
    if ix.program_id       != ED25519_ID                   ||  // The program id we expect
        !ix.accounts.is_empty()                            ||  // With no context accounts
        ix.data.len()       != (16 + 64 + 32 + msg.len())      // And data of this size
    {
        return Err(Brc20OracleError::InvalidSigner.into());
    }
    check_ed25519_data(&ix.data, pubkey, msg, sig)?; // If that's not the case, check data
    Ok(())
}

pub fn check_ed25519_data(data: &[u8], pubkey: &[u8], msg: &[u8], sig: &[u8]) -> ProgramResult {
    // According to this layout used by the Ed25519Program
    // https://github.com/solana-labs/solana-web3.js/blob/master/src/ed25519-program.ts#L33

    // "Deserializing" byte slices
    let num_signatures = &[data[0]]; // Byte  0
    let padding = &[data[1]]; // Byte  1
    let signature_offset = &data[2..=3]; // Bytes 2,3
    let signature_instruction_index = &data[4..=5]; // Bytes 4,5
    let public_key_offset = &data[6..=7]; // Bytes 6,7
    let public_key_instruction_index = &data[8..=9]; // Bytes 8,9
    let message_data_offset = &data[10..=11]; // Bytes 10,11
    let message_data_size = &data[12..=13]; // Bytes 12,13
    let message_instruction_index = &data[14..=15]; // Bytes 14,15

    let data_pubkey = &data[16..16 + 32]; // Bytes 16..16+32
    let data_sig = &data[48..48 + 64]; // Bytes 48..48+64
    let data_msg = &data[112..]; // Bytes 112..end

    // Expected values
    let exp_public_key_offset: u16 = 16; // 2*u8 + 7*u16
    let exp_signature_offset: u16 = exp_public_key_offset + pubkey.len() as u16;
    let exp_message_data_offset: u16 = exp_signature_offset + sig.len() as u16;
    let exp_num_signatures: u8 = 1;
    let exp_message_data_size: u16 = msg.len().try_into().unwrap();

    // Header and Arg Checks
    // Header
    if num_signatures != &exp_num_signatures.to_le_bytes()
        || padding != &[0]
        || signature_offset != &exp_signature_offset.to_le_bytes()
        || signature_instruction_index != &u16::MAX.to_le_bytes()
        || public_key_offset != &exp_public_key_offset.to_le_bytes()
        || public_key_instruction_index != &u16::MAX.to_le_bytes()
        || message_data_offset != &exp_message_data_offset.to_le_bytes()
        || message_data_size != &exp_message_data_size.to_le_bytes()
        || message_instruction_index != &u16::MAX.to_le_bytes()
    {
        return Err(Brc20OracleError::InvalidSigner.into());
    }

    // Arguments
    if data_pubkey != pubkey || data_msg != msg || data_sig != sig {
        return Err(Brc20OracleError::InvalidSigner.into());
    }
    Ok(())
}
