// Import Borsh traits for serializing/deserializing data to store on-chain
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Sealed};
use thiserror::Error;  // For creating custom error types with descriptions

// The account state structure that will be stored in the PDA
// This represents a restaurant review with rating and description
#[derive(BorshSerialize, BorshDeserialize)]
pub struct AccountState {
    pub is_initialized: bool,  // Whether this account has been initialized
    pub rating: u8,            // Restaurant rating (1-10)
    pub description: String,   // Review description/comment
    pub title: String,         // Restaurant name/title
}

// Sealed trait implementation - required by Solana's Pack trait
impl Sealed for AccountState {}

// Implement IsInitialized trait to check if account is ready to use
impl IsInitialized for AccountState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

// Custom error types for this program
// The #[error(...)] attributes provide user-friendly error messages
#[derive(Debug, Error)]
pub enum ReviewError {
    #[error("Account not initialized yet")]
    UninitializedAccount,

    #[error("PDA derived does not equal PDA passed in")]
    InvalidPDA,

    #[error("Rating greater than 10 or less than 1")]
    InvalidRating,
}

// Convert our custom errors into Solana's ProgramError type
// This allows our errors to be returned from instruction handlers
impl From<ReviewError> for ProgramError {
    fn from(e: ReviewError) -> Self {
        ProgramError::Custom(e as u32)  // Convert enum variant to error code
    }
}
