use transfer::process_instruction;

use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
    },
    solana_program_test::{processor, tokio, ProgramTest},
    solana_sdk::{signature::Signer, signer::keypair::Keypair, transaction::Transaction},
    solana_system_interface::instruction as system_instruction,
    spl_token::state::{Account, Mint},
    std::str::FromStr,
};

#[tokio::test]
async fn success() {
    // Enable Solana runtime logging to see msg!() output from the program
    solana_logger::setup_with_default("solana_runtime::message=debug");
    
    // Setup some pubkeys for the accounts
    let program_id = Pubkey::from_str("TransferTokens11111111111111111111111111111").unwrap();
    let source = Keypair::new();  // Token account that will hold tokens (owned by PDA)
    let mint = Keypair::new();    // The token mint account
    let destination = Keypair::new();  // Token account that will receive tokens (owned by payer)
    // Derive the PDA that will be the authority over the source account
    let (authority_pubkey, _) = Pubkey::find_program_address(&[b"authority"], &program_id);

    // Add the program to the test framework
    // This registers our program so when transactions are sent to program_id, 
    // they'll be handled by process_instruction
    let program_test = ProgramTest::new(
        "spl_example_transfer_tokens",  // Program name for logs
        program_id,                      // Program ID to listen for
        processor!(process_instruction), // Our program's instruction handler
    );
    let amount = 10_000;
    let decimals = 9;
    let rent = Rent::default();

    // Start the program test - creates a local test validator
    // Returns: banks_client (for interacting with accounts), payer (funded test account), recent_blockhash
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // STEP 1: Create and initialize the token mint
    // This calls the System Program and SPL Token Program (NOT our program)
    let transaction = Transaction::new_signed_with_payer(
        &[
            // Create the mint account with enough space for Mint data
            system_instruction::create_account(
                &payer.pubkey(),        // Funding account
                &mint.pubkey(),         // New account to create
                rent.minimum_balance(Mint::LEN),  // Lamports for rent exemption
                Mint::LEN as u64,       // Space needed for mint data
                &spl_token::id(),       // Owner program (SPL Token)
            ),
            // Initialize the mint with payer as mint authority
            spl_token::instruction::initialize_mint(
                &spl_token::id(),       // SPL Token program ID
                &mint.pubkey(),         // Mint account to initialize
                &payer.pubkey(),        // Mint authority (can create new tokens)
                None,                   // Freeze authority (optional)
                decimals,               // Number of decimal places
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),  // Transaction fee payer
        &[&payer, &mint],       // Signers: payer (fee payer, mint authority) and mint (being created)
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // STEP 2: Create and initialize the source token account (owned by PDA)
    // This calls the System Program and SPL Token Program (NOT our program)
    let transaction = Transaction::new_signed_with_payer(
        &[
            // Create the source account with enough space for token account data
            system_instruction::create_account(
                &payer.pubkey(),        // Funding account
                &source.pubkey(),       // New account to create
                rent.minimum_balance(Account::LEN),  // Lamports for rent exemption
                Account::LEN as u64,    // Space needed for token account data
                &spl_token::id(),       // Owner program (SPL Token)
            ),
            // Initialize as token account, owned by the PDA
            spl_token::instruction::initialize_account(
                &spl_token::id(),       // SPL Token program ID
                &source.pubkey(),       // Token account to initialize
                &mint.pubkey(),         // Which mint this account holds
                &authority_pubkey,      // Owner/authority of this token account (our PDA!)
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),  // Transaction fee payer
        &[&payer, &source],     // Signers: payer (fee payer) and source (being created)
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // STEP 3: Create and initialize the destination token account (owned by payer)
    // This calls the System Program and SPL Token Program (NOT our program)
    // The destination is owned by payer so they can control the received tokens
    let transaction = Transaction::new_signed_with_payer(
        &[
            // Create the destination account with enough space for token account data
            system_instruction::create_account(
                &payer.pubkey(),        // Funding account
                &destination.pubkey(),  // New account to create
                rent.minimum_balance(Account::LEN),  // Lamports for rent exemption
                Account::LEN as u64,    // Space needed for token account data
                &spl_token::id(),       // Owner program (SPL Token)
            ),
            // Initialize as token account, owned by the payer (normal user wallet)
            spl_token::instruction::initialize_account(
                &spl_token::id(),       // SPL Token program ID
                &destination.pubkey(),  // Token account to initialize
                &mint.pubkey(),         // Which mint this account holds
                &payer.pubkey(),        // Owner/authority of this token account (the user!)
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),      // Transaction fee payer
        &[&payer, &destination],    // Signers: payer (fee payer) and destination (being created)
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // STEP 4: Mint tokens to the source account (the PDA-owned account)
    // This calls the SPL Token Program (NOT our program)
    // Payer can mint because they were set as mint authority in step 1
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),       // SPL Token program ID
            &mint.pubkey(),         // Mint to create tokens from
            &source.pubkey(),       // Destination token account (receives new tokens)
            &payer.pubkey(),        // Mint authority (authorized to mint)
            &[],                    // No multisig signers
            amount,                 // Amount of tokens to mint
        )
        .unwrap()],
        Some(&payer.pubkey()),  // Transaction fee payer
        &[&payer],              // Signers: payer (fee payer and mint authority)
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // STEP 5: Call OUR program to transfer tokens from source to destination
    // THIS is the only transaction that calls our program!
    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_with_bincode(
            program_id,  // Our program's ID - routes to our process_instruction
            &(),         // Empty instruction data (our program doesn't use it)
            vec![
                AccountMeta::new(source.pubkey(), false),          // Writable, not signer
                AccountMeta::new_readonly(mint.pubkey(), false),   // Read-only, not signer
                AccountMeta::new(destination.pubkey(), false),     // Writable, not signer
                AccountMeta::new_readonly(authority_pubkey, false), // Read-only PDA, not signer (derived in program)
                AccountMeta::new_readonly(spl_token::id(), false), // SPL Token program to CPI into
            ],
        )],
        Some(&payer.pubkey()),  // Transaction fee payer
        &[&payer],              // Signers: only payer (pays fees)
        recent_blockhash,
    );

    // Execute the transaction - this will call our process_instruction function
    banks_client.process_transaction(transaction).await.unwrap();

    // STEP 6: Verify the transfer worked by checking destination account balance
    let account = banks_client
        .get_account(destination.pubkey())
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();
    assert_eq!(token_account.amount, 100);  // Should have all tokens from source
}