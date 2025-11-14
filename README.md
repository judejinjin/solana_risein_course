# Solana RiseIn Course

This repository contains Solana programs and applications developed during the RiseIn course.

## Projects

- **counter** - Basic counter program
- **CPI_Transfer** - Cross-Program Invocation transfer example
- **restaurant_review** - Restaurant review Solana program
- **review_frontend** - Next.js frontend for the restaurant review app

---

## Restaurant Review Program

A Solana program that allows users to add and update restaurant reviews on-chain. Each review is stored in a Program Derived Address (PDA) unique to the reviewer and restaurant combination.

### Features

- Add reviews with title, rating (1-10), and description
- Update existing reviews
- One review per restaurant per user (enforced via PDA)
- Secure ownership validation using PDAs

---

## Deploying to Solana Devnet

### Prerequisites

1. **Install Solana CLI** (if not already installed):
   ```bash
   sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
   ```

2. **Verify installation**:
   ```bash
   solana --version
   ```

3. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

### Deployment Steps

1. **Configure Solana CLI for devnet**:
   ```bash
   solana config set --url https://api.devnet.solana.com
   ```

2. **Create a new keypair** (or use existing):
   ```bash
   solana-keygen new --outfile ~/.config/solana/devnet-deployer.json
   ```

3. **Set the keypair as default**:
   ```bash
   solana config set --keypair ~/.config/solana/devnet-deployer.json
   ```

4. **Check your wallet address**:
   ```bash
   solana address
   ```

5. **Airdrop SOL for deployment fees** (devnet only):
   ```bash
   solana airdrop 2
   ```
   *Note: You may need to run this multiple times to get enough SOL for deployment.*

6. **Build the program**:
   ```bash
   cd restaurant_review
   cargo build-bpf
   ```
   
   Or with newer Solana versions:
   ```bash
   cargo build-sbf
   ```

7. **Deploy the program**:
   ```bash
   solana program deploy target/deploy/review.so
   ```

8. **Save the Program ID**:
   After deployment, you'll see output like:
   ```
   Program Id: <YOUR_PROGRAM_ID>
   ```
   **Important**: Copy this Program ID - you'll need it for the frontend configuration.

### Verify Deployment

Check that your program is deployed:
```bash
solana program show <YOUR_PROGRAM_ID>
```

---

## Running the Frontend

The `review_frontend` is a Next.js application that interacts with the deployed restaurant review program.

### Prerequisites

- Node.js (v16 or higher)
- npm or yarn
- A Solana wallet browser extension (Phantom, Solflare, etc.)

### Setup Steps

1. **Navigate to frontend directory**:
   ```bash
   cd review_frontend
   ```

2. **Install dependencies**:
   ```bash
   npm install
   ```
   Or with yarn:
   ```bash
   yarn install
   ```

3. **Configure the Program ID**:
   
   Update the program ID in the frontend code to match your deployed program. Look for files that reference the program ID (typically in `src/` directory) and update them with your deployed Program ID.

   Common locations:
   - `src/util/fetchReviews.ts`
   - `src/components/ReviewForm.tsx` (or similar)

4. **Configure Network** (if needed):
   
   Ensure the frontend is configured to use devnet. Look for RPC endpoint configurations and make sure they point to:
   ```
   https://api.devnet.solana.com
   ```

5. **Run the development server**:
   ```bash
   npm run dev
   ```
   Or with yarn:
   ```bash
   yarn dev
   ```

6. **Access the application**:
   
   Open your browser and navigate to:
   ```
   http://localhost:3000
   ```

7. **Connect your wallet**:
   - Ensure your wallet is set to **Devnet**
   - Click "Connect Wallet" in the application
   - Approve the connection request
   - Get some devnet SOL if needed: `solana airdrop 1` or use a devnet faucet

### Using the Application

1. **Add a Review**:
   - Enter restaurant title
   - Select a rating (1-10)
   - Write a description
   - Submit the transaction
   - Approve in your wallet

2. **Update a Review**:
   - Navigate to your existing review
   - Modify rating or description
   - Submit the update transaction
   - Approve in your wallet

3. **View Reviews**:
   - Browse all reviews on the platform
   - Filter by restaurant or reviewer

---

## Testing

### Run Program Tests

Navigate to the program directory and run tests:

```bash
cd restaurant_review
cargo test
```

This will run the comprehensive test suite including:
- Adding reviews with valid and invalid ratings
- Updating existing reviews
- Security tests (unauthorized updates)
- Multiple reviews per user
- Duplicate prevention

---

## Project Structure

```
solana/risein/
├── restaurant_review/          # Solana program
│   ├── src/
│   │   ├── lib.rs             # Main program logic
│   │   ├── instruction.rs     # Instruction parsing
│   │   └── state.rs           # Account state structures
│   ├── tests/
│   │   └── test.rs            # Integration tests
│   └── Cargo.toml
│
└── review_frontend/            # Next.js frontend
    ├── src/
    │   ├── pages/             # Next.js pages
    │   ├── components/        # React components
    │   ├── util/              # Utility functions
    │   └── models/            # Data models
    ├── package.json
    └── next.config.js
```

---

## Troubleshooting

### Deployment Issues

**Problem**: Insufficient SOL for deployment
```bash
# Solution: Request more airdrops
solana airdrop 2
# Check balance
solana balance
```

**Problem**: Program deployment fails
```bash
# Solution: Ensure you're on devnet
solana config get
# Should show: RPC URL: https://api.devnet.solana.com
```

### Frontend Issues

**Problem**: Cannot connect wallet
- Ensure wallet extension is installed
- Check that wallet is set to Devnet network
- Refresh the page and try again

**Problem**: Transactions failing
- Ensure you have enough SOL in your devnet wallet
- Verify the program ID is correctly configured
- Check browser console for error messages

**Problem**: Reviews not loading
- Verify the program is deployed and accessible
- Check that the correct RPC endpoint is configured
- Ensure the program ID matches your deployment

---

## Additional Resources

- [Solana Documentation](https://docs.solana.com/)
- [Solana Cookbook](https://solanacookbook.com/)
- [Anchor Framework](https://www.anchor-lang.com/)
- [Next.js Documentation](https://nextjs.org/docs)

---

## License

This project is for educational purposes as part of the RiseIn Solana course.
