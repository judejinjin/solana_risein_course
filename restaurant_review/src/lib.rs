// Module declarations - organize code into separate files
pub mod instruction;  // Instruction parsing and types
pub mod state;        // Account state structures and errors

use crate::instruction::ReviewInstruction;
use crate::state::AccountState;
use crate::state::ReviewError;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,  // Macro for logging on-chain
    program::invoke_signed,  // For CPI with PDA signing
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},  // For calculating rent exemption
};
use solana_system_interface::instruction as system_instruction;
use std::convert::TryInto;

// Define the program entrypoint
entrypoint!(process_instruction);

// Main entry point for all instructions sent to this program
pub fn process_instruction(
    program_id: &Pubkey,         // This program's ID
    accounts: &[AccountInfo],    // Accounts required by the instruction
    instruction_data: &[u8],     // Serialized instruction data
) -> ProgramResult {
    // Deserialize instruction data to determine which action to perform
    let instruction = ReviewInstruction::unpack(instruction_data)?;
    
    // Route to the appropriate handler based on instruction type
    match instruction {
        ReviewInstruction::AddReview {
            title,
            rating,
            description,
        } => add_review(program_id, accounts, title, rating, description),
        ReviewInstruction::UpdateReview {
            title,
            rating,
            description,
        } => update_review(program_id, accounts, title, rating, description),
    }
}

// Handler for adding a new restaurant review
// Creates a PDA account to store the review data
pub fn add_review(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    title: String,         // Restaurant name
    rating: u8,            // Rating 1-10
    description: String,   // Review text
) -> ProgramResult {
    msg!("Adding  review...");
    msg!("Title: {}", title);
    msg!("Rating: {}", rating);
    msg!("Description: {}", description);

    let account_info_iter = &mut accounts.iter();

    // Expected accounts in order:
    let initializer = next_account_info(account_info_iter)?;   // User creating the review (signer)
    let pda_account = next_account_info(account_info_iter)?;   // PDA to store review data
    let system_program = next_account_info(account_info_iter)?; // System program for account creation

    // Verify the user has signed the transaction
    if !initializer.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive the PDA address using initializer pubkey and title as seeds
    // This ensures each user can only have one review per restaurant title
    let (pda, bump_seed) = Pubkey::find_program_address(
        &[initializer.key.as_ref(), title.as_bytes().as_ref()],
        program_id,
    );
    
    // Verify the PDA account passed in matches our derived address
    if pda != *pda_account.key {
        msg!("Invalid seeds for PDA");
        return Err(ProgramError::InvalidArgument);
    }

    // Validate rating is within acceptable range
    if rating > 10 || rating < 1 {
        return Err(ReviewError::InvalidRating.into());
    }

    // Fixed account size for storing review data
    // In production, you'd calculate this based on actual data size
    let account_len: usize = 1000;

    // Calculate rent-exempt minimum balance required
    // Note: In tests, Rent::get() may fail with UnsupportedSysvar
    // Using Rent::default() provides standard rent parameters
    let rent = Rent::default();
    let rent_lamports = rent.minimum_balance(account_len);

    // Create the PDA account via CPI to System Program
    // invoke_signed allows our PDA to "sign" the transaction
    invoke_signed(
        &system_instruction::create_account(
            initializer.key,     // Funding account
            pda_account.key,     // New account to create
            rent_lamports,       // Lamports for rent exemption
            account_len.try_into().unwrap(),  // Account size in bytes
            program_id,          // Owner of the new account (this program)
        ),
        &[
            initializer.clone(),
            pda_account.clone(),
            system_program.clone(),
        ],
        // PDA seeds for signing: [user_pubkey, title, bump_seed]
        &[&[
            initializer.key.as_ref(),
            title.as_bytes().as_ref(),
            &[bump_seed],
        ]],
    )?;

    msg!("PDA created: {}", pda);

    // Create a new AccountState with the review data
    // For a newly created account, we start fresh rather than deserializing zeros
    msg!("Creating account state");
    let mut account_data = AccountState {
        title,
        rating,
        description,
        is_initialized: true,
    };

    msg!("serializing account");
    // Serialize the updated state back to the account
    account_data.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;
    msg!("state account serialized");

    Ok(())
}

// Handler for updating an existing restaurant review
// Only allows the original reviewer to update their review
pub fn update_review(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _title: String,        // Title is used to derive PDA, but not changed
    rating: u8,            // New rating
    description: String,   // New description
) -> ProgramResult {
    msg!("Updating  review...");

    let account_info_iter = &mut accounts.iter();

    // Expected accounts:
    let initializer = next_account_info(account_info_iter)?;  // Original reviewer (signer)
    let pda_account = next_account_info(account_info_iter)?;  // Existing review PDA

    // Verify the PDA is owned by this program
    if pda_account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    // Verify the original reviewer is signing
    if !initializer.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    msg!("unpacking state account");
    // Deserialize existing review data
    // Use deserialize to handle accounts larger than the serialized data
    let mut account_data =
        AccountState::deserialize(&mut &pda_account.data.borrow()[..])
            .map_err(|_| ProgramError::InvalidAccountData)?;
    msg!("review title: {}", account_data.title);

    // Re-derive the PDA to verify the signer is the original reviewer
    // Uses the stored title and the signer's pubkey
    let (pda, _bump_seed) = Pubkey::find_program_address(
        &[
            initializer.key.as_ref(),
            account_data.title.as_bytes().as_ref(),
        ],
        program_id,
    );
    
    // Ensure the PDA matches (proves this user created the review)
    if pda != *pda_account.key {
        msg!("Invalid seeds for PDA");
        return Err(ReviewError::InvalidPDA.into());
    }

    msg!("checking if  account is initialized");
    // Can't update a review that doesn't exist
    if !account_data.is_initialized() {
        msg!("Account is not initialized");
        return Err(ReviewError::UninitializedAccount.into());
    }

    // Validate new rating
    if rating > 10 || rating < 1 {
        return Err(ReviewError::InvalidRating.into());
    }

    msg!("Review before update:");
    msg!("Title: {}", account_data.title);
    msg!("Rating: {}", account_data.rating);
    msg!("Description: {}", account_data.description);

    // Update only the rating and description (title stays the same)
    account_data.rating = rating;
    account_data.description = description;

    msg!("Review after update:");
    msg!("Title: {}", account_data.title);
    msg!("Rating: {}", account_data.rating);
    msg!("Description: {}", account_data.description);

    msg!("serializing account");
    // Save the updated state back to the account
    account_data.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;
    msg!("state account serialized");

    Ok(())
}
