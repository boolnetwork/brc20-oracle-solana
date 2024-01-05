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
use types::*;
use error::Brc20OracleError;

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

const COMMITTEE_PREFIX: &[u8] = b"Brc20OracleCommittee";
const ASSET_PREFIX: &[u8] = b"Brc20OracleAsset";

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = Brc20OracleInstruction::try_from_slice(instruction_data)?;
    match instruction {
        Brc20OracleInstruction::SetCommittee(committee) => set_committee(program_id, accounts, committee),
        Brc20OracleInstruction::Request(key) => request(program_id, accounts, key),
        Brc20OracleInstruction::Insert(key, amount) => insert(program_id, accounts, key, amount),
    }
}

pub fn set_committee(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    committee: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer_info = next_account_info(account_info_iter)?;
    let committee_info = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    let (committee_address, bump) = Pubkey::find_program_address(
        &[COMMITTEE_PREFIX],
        program_id,
    );
    if committee_info.key != &committee_address {
        return Err(Brc20OracleError::IncorrectCommitteePDA.into());
    }

    let parse_committee = Pubkey::try_from_slice(&committee_info.data.borrow());
    match parse_committee {
        Ok(brc20_committee) => {
            if &brc20_committee != payer_info.key {
                return Err(Brc20OracleError::NotSignedByCommittee.into());
            }
            if committee_info.owner != program_id {
                return Err(Brc20OracleError::NotOwnedByBrc20Oracle.into());
            }
        }
        Err(_) => {
            invoke_signed(
                &system_instruction::create_account(
                    payer_info.key,
                    &committee_info.key,
                    Rent::get()?.minimum_balance(32),
                    32,
                    program_id,
                ),
                &[payer_info.clone(), committee_info.clone(), system_program.clone()],
                &[&[COMMITTEE_PREFIX, &[bump]]],
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
        &[ASSET_PREFIX, key.try_to_vec()?.as_slice()],
        program_id,
    );
    if &asset_address != brc20_asset_info.key {
        return Err(Brc20OracleError::IncorrectAssetPDA.into());
    }
    let parse_amount = Brc20Asset::try_from_slice(&brc20_asset_info.data.borrow());
    match parse_amount {
        Ok(_) => return Err(Brc20OracleError::DuplicateRequest.into()),
        Err(_) => {
            let asset = Brc20Asset { key: key.clone(), amount: 0 };
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
                &[&[ASSET_PREFIX, key.try_to_vec()?.as_slice(), &[bump]]],
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
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer_info = next_account_info(account_info_iter)?;
    let committee_info = next_account_info(account_info_iter)?;
    let brc20_asset_info = next_account_info(account_info_iter)?;
    // check committee info's correctness.
    if committee_info.owner != program_id {
        return Err(Brc20OracleError::NotOwnedByBrc20Oracle.into());
    }
    let (committee_info_address, _) = Pubkey::find_program_address(&[COMMITTEE_PREFIX], program_id);
    if &committee_info_address != committee_info.key {
        return Err(Brc20OracleError::IncorrectCommitteePDA.into());
    }
    // ensure that payer is committee.
    if !payer_info.is_signer {
        return Err(Brc20OracleError::PayerNotSigner.into());
    }
    let committee = Pubkey::try_from_slice(&committee_info.data.borrow())?;
    if payer_info.key != &committee {
        return Err(Brc20OracleError::PayerNotCommittee.into());
    }
    // check corresponding amount address's correctness.
    let (asset_address, _) = Pubkey::find_program_address(
        &[ASSET_PREFIX, key.try_to_vec()?.as_slice()],
        program_id,
    );
    if &asset_address != brc20_asset_info.key {
        return Err(Brc20OracleError::IncorrectAssetPDA.into());
    }
    // check corresponding amount's owner and if it's initialized.
    if brc20_asset_info.owner != program_id {
        return Err(Brc20OracleError::NotOwnedByBrc20Oracle.into());
    }
    let asset = Brc20Asset::try_from_slice(&brc20_asset_info.data.borrow()) ;
    match asset {
        Ok(_) => {
            let new_asset = Brc20Asset { key, amount };
            new_asset.serialize(&mut &mut brc20_asset_info.data.borrow_mut()[..])?;
            msg!("update asset: {:?}", new_asset);
        },
        Err(_) => return Err(Brc20OracleError::RequestNotInitialized.into())
    }

    Ok(())
}
