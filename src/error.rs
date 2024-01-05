use num_derive::FromPrimitive;
use thiserror::Error;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum Brc20OracleError {
    #[error("Incorrect committee PDA")]
    IncorrectCommitteePDA,
    #[error("Incorrect Brc20 asset PDA")]
    IncorrectAssetPDA,
    #[error("Not signed by committee")]
    NotSignedByCommittee,
    #[error("Not owned by this Brc20 Oracle Program")]
    NotOwnedByBrc20Oracle,
    #[error("Duplicate request for this data")]
    DuplicateRequest,
    #[error("Brc20 request not initialized")]
    RequestNotInitialized,
    #[error("Payer isn't signer")]
    PayerNotSigner,
    #[error("Payer isn't committee account")]
    PayerNotCommittee,
}

impl From<Brc20OracleError> for ProgramError {
    fn from(e: Brc20OracleError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for Brc20OracleError {
    fn type_of() -> &'static str {
        "Brc20OracleError"
    }
}

impl PrintProgramError for Brc20OracleError {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}
