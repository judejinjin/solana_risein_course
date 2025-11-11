use {
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke_signed,  // Used for Cross-Program Invocation (CPI) with PDA signing
        program_error::ProgramError,
        program_pack::Pack,      // Trait for unpacking account data
        pubkey::Pubkey,
    },
    spl_token::{
        instruction::transfer_checked,  // SPL Token instruction builder
        state::{Account, Mint},         // SPL Token account structures
    },
};

// Define the program entrypoint - this macro sets up the entry function
// that the Solana runtime calls when a transaction is sent to this program
solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,         // The program ID of THIS program
    accounts: &[AccountInfo],    // All accounts passed to this instruction
    _instruction_data: &[u8],    // Instruction data (unused in this program)
) -> ProgramResult {
    // Create an iterator to safely reference accounts in the slice
    let account_info_iter = &mut accounts.iter();

    // Extract accounts in the order specified by the instruction
    // These match the order defined in the test file (line 144-148)
    let source_info = next_account_info(account_info_iter)?;      // 1. Source token account (owned by PDA)
    let mint_info = next_account_info(account_info_iter)?;        // 2. Token mint
    let destination_info = next_account_info(account_info_iter)?; // 3. Destination token account (owned by user)
    let authority_info = next_account_info(account_info_iter)?;   // 4. PDA authority (not a signer, derived)
    let token_program_info = next_account_info(account_info_iter)?; // 5. SPL Token program (for CPI)

    // Verify that the authority account is the correct PDA
    // We derive the PDA using the same seed that was used to create it
    let (expected_authority, bump_seed) = Pubkey::find_program_address(&[b"authority"], program_id);
    if expected_authority != *authority_info.key {
        return Err(ProgramError::InvalidSeeds);  // Reject if PDA doesn't match
    }

    // Unpack the source token account to read its data
    // This deserializes the raw account data into an SPL Token Account struct
    let source_account = Account::unpack(&source_info.try_borrow_data()?)?;
    // let amount = source_account.amount;  // Get all tokens from the source account
    let amount = 100; // homework #2 change to hardcoded value for transfer

    // Unpack the mint account to get the decimal configuration
    // transfer_checked requires decimals to prevent precision errors
    let mint = Mint::unpack(&mint_info.try_borrow_data()?)?;
    let decimals = mint.decimals;

    // Log the transfer attempt (visible in program logs when enabled)
    msg!("Attempting to transfer {} tokens", amount);
    
    // Perform a Cross-Program Invocation (CPI) to the SPL Token program
    // invoke_signed allows our PDA to "sign" even though it has no private key
    invoke_signed(
        // Build the transfer_checked instruction for SPL Token program
        &transfer_checked(
            token_program_info.key,   // SPL Token program ID
            source_info.key,          // Source token account (from)
            mint_info.key,            // Token mint (for verification)
            destination_info.key,     // Destination token account (to)
            authority_info.key,       // Authority (our PDA that owns source account)
            &[],                      // No multisig signers
            amount,                   // Amount to transfer
            decimals,                 // Decimals (prevents precision errors)
        )
        .unwrap(),
        // Accounts required by the SPL Token program for this instruction
        // Must be in the order expected by transfer_checked
        &[
            source_info.clone(),      // Source token account
            mint_info.clone(),        // Mint account
            destination_info.clone(), // Destination token account
            authority_info.clone(),   // Authority (PDA)
            token_program_info.clone(), // SPL Token program itself (not required, but good practice)
        ],
        // PDA seeds to "sign" the transaction
        // The outer array allows multiple PDAs, inner arrays contain [seed, bump] for each PDA
        &[&[b"authority", &[bump_seed]]],  // Our PDA: seed="authority" + bump_seed
    )
}