use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;

// Define the instructions this program can handle
// Each instruction represents an action users can perform
pub enum ReviewInstruction {
    // Create a new restaurant review
    AddReview {
        title: String,       // Restaurant name
        rating: u8,          // Rating from 1-10
        description: String, // Review text
    },
    // Update an existing restaurant review
    UpdateReview {
        title: String,       // Restaurant name (used to find the PDA)
        rating: u8,          // New rating
        description: String, // New review text
    },
}

// Internal structure for deserializing instruction data
// This matches the data format sent by clients
#[derive(BorshDeserialize)]
struct ReviewPayload {
    title: String,
    rating: u8,
    description: String,
}

impl ReviewInstruction {
    // Deserialize instruction data from bytes into a ReviewInstruction enum
    // Instruction format: [variant_byte][borsh_serialized_payload]
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        // First byte indicates which instruction variant (0 = AddReview, 1 = UpdateReview)
        let (&variant, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;
        
        // Remaining bytes contain the instruction data (title, rating, description)
        let payload = ReviewPayload::try_from_slice(rest)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
        
        // Match on variant to create the appropriate instruction
        Ok(match variant {
            0 => Self::AddReview {
                title: payload.title,
                rating: payload.rating,
                description: payload.description,
            },
            1 => Self::UpdateReview {
                title: payload.title,
                rating: payload.rating,
                description: payload.description,
            },
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}
