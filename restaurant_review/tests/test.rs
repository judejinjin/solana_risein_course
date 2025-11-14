// Integration tests for the restaurant review Solana program
// These tests use solana-program-test to simulate on-chain behavior

use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use review::state::AccountState;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use solana_system_interface::instruction as system_instruction;
use std::str::FromStr;

// System program ID constant - used for account creation and transfers
const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";

/// Payload structure for AddReview instruction
/// Must match the structure expected by ReviewInstruction::unpack in instruction.rs
#[derive(borsh::BorshSerialize)]
struct AddReviewPayload {
    title: String,
    rating: u8,
    description: String,
}

/// Payload structure for UpdateReview instruction
/// Must match the structure expected by ReviewInstruction::unpack in instruction.rs
#[derive(borsh::BorshSerialize)]
struct UpdateReviewPayload {
    title: String,
    rating: u8,
    description: String,
}

/// Helper function to create instruction data for AddReview
/// Format: [variant_byte: 0][borsh_serialized_payload]
/// The variant byte (0) indicates this is an AddReview instruction
fn create_add_review_instruction_data(title: &str, rating: u8, description: &str) -> Vec<u8> {
    let mut data = vec![0u8]; // Variant 0 for AddReview
    
    let payload = AddReviewPayload {
        title: title.to_string(),
        rating,
        description: description.to_string(),
    };
    
    // Append the Borsh-serialized payload after the variant byte
    data.extend_from_slice(&borsh::to_vec(&payload).unwrap());
    data
}

/// Helper function to create instruction data for UpdateReview
/// Format: [variant_byte: 1][borsh_serialized_payload]
/// The variant byte (1) indicates this is an UpdateReview instruction
fn create_update_review_instruction_data(title: &str, rating: u8, description: &str) -> Vec<u8> {
    let mut data = vec![1u8]; // Variant 1 for UpdateReview
    
    let payload = UpdateReviewPayload {
        title: title.to_string(),
        rating,
        description: description.to_string(),
    };
    
    // Append the Borsh-serialized payload after the variant byte
    data.extend_from_slice(&borsh::to_vec(&payload).unwrap());
    data
}

/// TEST 1: Successfully add a restaurant review
/// 
/// This test verifies the complete happy path for adding a review:
/// 1. Sets up the test environment with a program instance
/// 2. Creates and funds a reviewer account
/// 3. Creates a review PDA (Program Derived Address) for storage
/// 4. Submits an AddReview instruction
/// 5. Verifies the review data was stored correctly on-chain
#[tokio::test]
async fn test_add_review_success() {
    // Enable Solana runtime logging to see msg!() output from the program
    solana_logger::setup_with_default("solana_runtime::message=debug");
    
    // Setup program and accounts
    // The program ID must be a valid base58-encoded public key
    let program_id = Pubkey::from_str("Fm4FXYj8mbBzHnwq1V7Yh5cP9TqrGJSqYdHZ3u2KLxRV").unwrap();
    let mut program_test = ProgramTest::new(
        "review",                              // Program name
        program_id,                            // Program ID
        processor!(review::process_instruction), // Entry point function
    );
    
    // Start test environment - creates a local validator with our program deployed
    // Returns: banks_client (for transactions), payer (funded account), recent_blockhash
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Create a new keypair for the user who will write the review
    let reviewer = Keypair::new();
    
    // Fund the reviewer account so they can pay for PDA creation
    // The reviewer needs SOL to pay rent for the new account
    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &payer.pubkey(),
            &reviewer.pubkey(),
            10_000_000, // 0.01 SOL (lamports)
        )],
        Some(&payer.pubkey()),  // Fee payer
        &[&payer],              // Signers
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
    
    // Restaurant review details
    let title = "Best Pizza Place";
    let rating = 9u8;
    let description = "Amazing pizza with great service!";
    
    // Derive PDA for this review
    // The PDA is derived from [reviewer_pubkey, title] ensuring each user
    // can only have one review per restaurant title
    let (pda, _bump) = Pubkey::find_program_address(
        &[reviewer.pubkey().as_ref(), title.as_bytes()],
        &program_id,
    );
    
    // Create the AddReview instruction data
    let instruction_data = create_add_review_instruction_data(title, rating, description);
    
    // Build the instruction with required accounts
    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new(reviewer.pubkey(), true),  // Reviewer (signer, pays rent)
            AccountMeta::new(pda, false),                // PDA account to create (writable)
            AccountMeta::new_readonly(Pubkey::from_str(SYSTEM_PROGRAM_ID).unwrap(), false), // System program
        ],
    );
    
    // Create and send transaction
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),  // Fee payer
        &[&payer, &reviewer],   // Signers: payer for fees, reviewer for instruction
        recent_blockhash,
    );
    
    // Process transaction - should succeed
    let result = banks_client.process_transaction(transaction).await;
    if let Err(e) = &result {
        panic!("Transaction failed: {:?}", e);
    }
    result.unwrap();
    
    // Verify the account was created and data is correct
    let account = banks_client
        .get_account(pda)
        .await
        .unwrap()
        .expect("PDA account should exist");
    
    // Deserialize and verify the stored data
    // Use deserialize() instead of try_from_slice() to handle accounts
    // larger than the serialized data (account is 1000 bytes, data is smaller)
    let account_state = AccountState::deserialize(&mut &account.data[..]).unwrap();
    assert_eq!(account_state.is_initialized, true);
    assert_eq!(account_state.title, title);
    assert_eq!(account_state.rating, rating);
    assert_eq!(account_state.description, description);
}

/// TEST 2: Reject review with rating too high
/// 
/// Tests input validation - the program should reject ratings above 10.
/// This ensures data integrity and prevents invalid ratings from being stored.
/// The program validates: 1 <= rating <= 10
#[tokio::test]
async fn test_add_review_invalid_rating_too_high() {
    solana_logger::setup_with_default("solana_runtime::message=debug");
    
    let program_id = Pubkey::from_str("Fm4FXYj8mbBzHnwq1V7Yh5cP9TqrGJSqYdHZ3u2KLxRV").unwrap();
    let mut program_test = ProgramTest::new(
        "review",
        program_id,
        processor!(review::process_instruction),
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    let reviewer = Keypair::new();
    
    // Fund the reviewer account
    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &payer.pubkey(),
            &reviewer.pubkey(),
            10_000_000,
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
    
    let title = "Test Restaurant";
    let rating = 11u8; // Invalid: too high (max is 10)
    let description = "Test description";
    
    let (pda, _bump) = Pubkey::find_program_address(
        &[reviewer.pubkey().as_ref(), title.as_bytes()],
        &program_id,
    );
    
    let instruction_data = create_add_review_instruction_data(title, rating, description);
    
    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new(reviewer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new_readonly(Pubkey::from_str(SYSTEM_PROGRAM_ID).unwrap(), false),
        ],
    );
    
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer, &reviewer],
        recent_blockhash,
    );
    
    // Should fail due to invalid rating (> 10)
    // The program checks: if rating > 10 { return InvalidRating error }
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err(), "Transaction should fail with rating > 10");
}

/// TEST 3: Reject review with rating too low
/// 
/// Tests the lower bound of rating validation - ratings must be at least 1.
/// This prevents zero or negative ratings which wouldn't make sense for reviews.
#[tokio::test]
async fn test_add_review_invalid_rating_too_low() {
    solana_logger::setup_with_default("solana_runtime::message=debug");
    
    let program_id = Pubkey::from_str("Fm4FXYj8mbBzHnwq1V7Yh5cP9TqrGJSqYdHZ3u2KLxRV").unwrap();
    let mut program_test = ProgramTest::new(
        "review",
        program_id,
        processor!(review::process_instruction),
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    let reviewer = Keypair::new();
    
    // Fund the reviewer account
    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &payer.pubkey(),
            &reviewer.pubkey(),
            10_000_000,
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
    
    let title = "Test Restaurant";
    let rating = 0u8; // Invalid: too low (min is 1)
    let description = "Test description";
    
    let (pda, _bump) = Pubkey::find_program_address(
        &[reviewer.pubkey().as_ref(), title.as_bytes()],
        &program_id,
    );
    
    let instruction_data = create_add_review_instruction_data(title, rating, description);
    
    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new(reviewer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new_readonly(Pubkey::from_str(SYSTEM_PROGRAM_ID).unwrap(), false),
        ],
    );
    
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer, &reviewer],
        recent_blockhash,
    );
    
    // Should fail due to invalid rating (< 1)
    // The program checks: if rating < 1 { return InvalidRating error }
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err(), "Transaction should fail with rating < 1");
}

/// TEST 4: Successfully update an existing review
/// 
/// This test demonstrates the complete update flow:
/// 1. User creates an initial review
/// 2. User later updates the review with new rating and description
/// 3. The title cannot be changed (it's part of the PDA derivation)
/// 4. Only the original reviewer can update (verified via PDA seeds)
#[tokio::test]
async fn test_update_review_success() {
    solana_logger::setup_with_default("solana_runtime::message=debug");
    
    let program_id = Pubkey::from_str("Fm4FXYj8mbBzHnwq1V7Yh5cP9TqrGJSqYdHZ3u2KLxRV").unwrap();
    let mut program_test = ProgramTest::new(
        "review",
        program_id,
        processor!(review::process_instruction),
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    let reviewer = Keypair::new();
    
    // Fund the reviewer account
    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &payer.pubkey(),
            &reviewer.pubkey(),
            10_000_000,
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
    
    let title = "Pasta House";
    let initial_rating = 7u8;
    let initial_description = "Good pasta";
    
    // Derive the same PDA for both add and update operations
    let (pda, _bump) = Pubkey::find_program_address(
        &[reviewer.pubkey().as_ref(), title.as_bytes()],
        &program_id,
    );
    
    // STEP 1: Add initial review
    let add_instruction_data = create_add_review_instruction_data(title, initial_rating, initial_description);
    
    let add_instruction = Instruction::new_with_bytes(
        program_id,
        &add_instruction_data,
        vec![
            AccountMeta::new(reviewer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new_readonly(Pubkey::from_str(SYSTEM_PROGRAM_ID).unwrap(), false),
        ],
    );
    
    let add_transaction = Transaction::new_signed_with_payer(
        &[add_instruction],
        Some(&payer.pubkey()),
        &[&payer, &reviewer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(add_transaction).await.unwrap();
    
    // STEP 2: Update the review with new rating and description
    let updated_rating = 9u8;
    let updated_description = "Actually amazing pasta! Changed my mind.";
    
    let update_instruction_data = create_update_review_instruction_data(title, updated_rating, updated_description);
    
    // Note: Update instruction doesn't need System Program account
    // The PDA already exists, we're just modifying its data
    let update_instruction = Instruction::new_with_bytes(
        program_id,
        &update_instruction_data,
        vec![
            AccountMeta::new(reviewer.pubkey(), true),  // Original reviewer (must sign)
            AccountMeta::new(pda, false),                // Existing PDA account (writable)
        ],
    );
    
    let update_transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&payer.pubkey()),
        &[&payer, &reviewer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(update_transaction).await.unwrap();
    
    // STEP 3: Verify the update worked
    let account = banks_client
        .get_account(pda)
        .await
        .unwrap()
        .expect("PDA account should exist");
    
    let account_state = AccountState::deserialize(&mut &account.data[..]).unwrap();
    assert_eq!(account_state.is_initialized, true);
    assert_eq!(account_state.title, title); // Title doesn't change (part of PDA seeds)
    assert_eq!(account_state.rating, updated_rating);
    assert_eq!(account_state.description, updated_description);
}

/// TEST 5: Prevent unauthorized updates
/// 
/// Security test: Ensures that only the original reviewer can update their review.
/// The PDA derivation uses [reviewer_pubkey, title] as seeds, so if a different
/// user tries to update, the PDA address won't match and the transaction fails.
/// This demonstrates Solana's PDA-based access control pattern.
#[tokio::test]
async fn test_update_review_wrong_reviewer_fails() {
    solana_logger::setup_with_default("solana_runtime::message=debug");
    
    let program_id = Pubkey::from_str("Fm4FXYj8mbBzHnwq1V7Yh5cP9TqrGJSqYdHZ3u2KLxRV").unwrap();
    let mut program_test = ProgramTest::new(
        "review",
        program_id,
        processor!(review::process_instruction),
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    let original_reviewer = Keypair::new();
    let malicious_user = Keypair::new(); // Different user attempting unauthorized update
    
    // Fund the original reviewer account
    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &payer.pubkey(),
            &original_reviewer.pubkey(),
            10_000_000,
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
    
    let title = "Secure Restaurant";
    let rating = 8u8;
    let description = "Original review";
    
    // PDA derived from ORIGINAL reviewer's pubkey
    // This creates a unique address owned by the original reviewer
    let (pda, _bump) = Pubkey::find_program_address(
        &[original_reviewer.pubkey().as_ref(), title.as_bytes()],
        &program_id,
    );
    
    // STEP 1: Original reviewer creates the review
    let add_instruction_data = create_add_review_instruction_data(title, rating, description);
    
    let add_instruction = Instruction::new_with_bytes(
        program_id,
        &add_instruction_data,
        vec![
            AccountMeta::new(original_reviewer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new_readonly(Pubkey::from_str(SYSTEM_PROGRAM_ID).unwrap(), false),
        ],
    );
    
    let add_transaction = Transaction::new_signed_with_payer(
        &[add_instruction],
        Some(&payer.pubkey()),
        &[&payer, &original_reviewer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(add_transaction).await.unwrap();
    
    // STEP 2: Malicious user tries to update (should fail)
    let malicious_description = "Hacked review!";
    
    let update_instruction_data = create_update_review_instruction_data(title, 1, malicious_description);
    
    let update_instruction = Instruction::new_with_bytes(
        program_id,
        &update_instruction_data,
        vec![
            AccountMeta::new(malicious_user.pubkey(), true), // Wrong signer!
            AccountMeta::new(pda, false),
        ],
    );
    
    let update_transaction = Transaction::new_signed_with_payer(
        &[update_instruction],
        Some(&payer.pubkey()),
        &[&payer, &malicious_user],
        recent_blockhash,
    );
    
    // Should fail because when the program re-derives the PDA using
    // malicious_user.pubkey() + title, it won't match the provided PDA address
    // The program will return InvalidPDA error
    let result = banks_client.process_transaction(update_transaction).await;
    assert!(result.is_err(), "Should not allow different user to update review");
    
    // Verify original review is unchanged
    let account = banks_client
        .get_account(pda)
        .await
        .unwrap()
        .expect("PDA account should exist");
    
    let account_state = AccountState::deserialize(&mut &account.data[..]).unwrap();
    assert_eq!(account_state.description, description); // Still original description
}

/// TEST 6: Multiple PDAs for different restaurants
/// 
/// Demonstrates that the PDA derivation using [reviewer_pubkey, title] allows
/// a single user to create multiple reviews for different restaurants. Each
/// (user, restaurant) pair generates a unique PDA address, enabling one review
/// per restaurant while preventing duplicates for the same restaurant.
#[tokio::test]
async fn test_multiple_reviews_same_user_different_restaurants() {
    solana_logger::setup_with_default("solana_runtime::message=debug");
    
    let program_id = Pubkey::from_str("Fm4FXYj8mbBzHnwq1V7Yh5cP9TqrGJSqYdHZ3u2KLxRV").unwrap();
    let mut program_test = ProgramTest::new(
        "review",
        program_id,
        processor!(review::process_instruction),
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    let reviewer = Keypair::new();
    
    // Fund the reviewer account (needs sufficient lamports for multiple PDA creations)
    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &payer.pubkey(),
            &reviewer.pubkey(),
            30_000_000, // More funds for multiple transactions
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
    
    // Three different restaurants to review
    // Each title is a seed component, so each generates a unique PDA
    let reviews = vec![
        ("Pizza Place", 9u8, "Great pizza"),
        ("Burger Joint", 7u8, "Good burgers"),
        ("Sushi Bar", 10u8, "Best sushi ever"),
    ];
    
    // Create a review for each restaurant (same user, different titles = different PDAs)
    for (title, rating, description) in reviews.iter() {
        let (pda, _bump) = Pubkey::find_program_address(
            &[reviewer.pubkey().as_ref(), title.as_bytes()],
            &program_id,
        );
        
        let instruction_data = create_add_review_instruction_data(title, *rating, description);
        
        let instruction = Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(reviewer.pubkey(), true),
                AccountMeta::new(pda, false),
                AccountMeta::new_readonly(Pubkey::from_str(SYSTEM_PROGRAM_ID).unwrap(), false),
            ],
        );
        
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
            &[&payer, &reviewer],
            recent_blockhash,
        );
        
        banks_client.process_transaction(transaction).await.unwrap();
        
        // Verify each review is stored correctly with unique PDA
        let account = banks_client.get_account(pda).await.unwrap().unwrap();
        let account_state = AccountState::deserialize(&mut &account.data[..]).unwrap();
        assert_eq!(account_state.title, *title);
        assert_eq!(account_state.rating, *rating);
        assert_eq!(account_state.description, *description);
    }
}

/// TEST 7: Prevent duplicate reviews for the same restaurant
/// 
/// Ensures that the same user cannot create multiple reviews for the same restaurant.
/// Since PDA is derived from [reviewer_pubkey, title], attempting to create a second
/// review with the same title will try to initialize an already-initialized account,
/// which Solana prevents. This enforces the business rule: one review per restaurant
/// per user. To modify a review, users must use the update instruction instead.
#[tokio::test]
async fn test_cannot_add_duplicate_review() {
    solana_logger::setup_with_default("solana_runtime::message=debug");
    
    let program_id = Pubkey::from_str("Fm4FXYj8mbBzHnwq1V7Yh5cP9TqrGJSqYdHZ3u2KLxRV").unwrap();
    let mut program_test = ProgramTest::new(
        "review",
        program_id,
        processor!(review::process_instruction),
    );
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    let reviewer = Keypair::new();
    
    // Fund the reviewer account with enough lamports for multiple attempts
    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &payer.pubkey(),
            &reviewer.pubkey(),
            20_000_000, // Extra funds for potential second attempt
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
    
    let title = "Duplicate Test";
    let rating = 5u8;
    let description = "First review";
    
    // Same PDA will be derived for both attempts since same user + same title
    let (pda, _bump) = Pubkey::find_program_address(
        &[reviewer.pubkey().as_ref(), title.as_bytes()],
        &program_id,
    );
    
    // STEP 1: Create first review (should succeed)
    let instruction_data = create_add_review_instruction_data(title, rating, description);
    
    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new(reviewer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new_readonly(Pubkey::from_str(SYSTEM_PROGRAM_ID).unwrap(), false),
        ],
    );
    
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer, &reviewer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(transaction).await.unwrap();
    
    // STEP 2: Try to create duplicate review with same title (should fail)
    // Same title means same PDA address - but that account is already initialized!
    let duplicate_description = "Trying to add again";
    let instruction_data2 = create_add_review_instruction_data(title, rating, duplicate_description);
    
    let instruction2 = Instruction::new_with_bytes(
        program_id,
        &instruction_data2,
        vec![
            AccountMeta::new(reviewer.pubkey(), true),
            AccountMeta::new(pda, false), // Same PDA as before
            AccountMeta::new_readonly(Pubkey::from_str(SYSTEM_PROGRAM_ID).unwrap(), false),
        ],
    );
    
    let transaction2 = Transaction::new_signed_with_payer(
        &[instruction2],
        Some(&payer.pubkey()),
        &[&payer, &reviewer],
        recent_blockhash,
    );
    
    // Should fail because:
    // 1. Same reviewer + same title = same PDA
    // 2. That PDA account is already initialized
    // 3. Solana prevents re-initializing an account
    // Result: Duplicate reviews are impossible; users must use update instead
    let result = banks_client.process_transaction(transaction2).await;
    assert!(result.is_err(), "Should not allow duplicate review for same restaurant");
}
