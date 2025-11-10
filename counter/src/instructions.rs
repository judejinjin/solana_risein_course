use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;

#[derive(Debug, BorshDeserialize, BorshSerialize)]
pub struct CounterArgs {
    pub value: u32,
}

pub enum CounterInstructions {
    Increment(CounterArgs),
    Decrement(CounterArgs),
    Update(CounterArgs),
    Reset,
}

impl CounterInstructions {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&variant, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        Ok(match variant {
            0 => Self::Increment(CounterArgs::try_from_slice(rest).unwrap()),
            1 => Self::Decrement(CounterArgs::try_from_slice(rest).unwrap()),
            2 => Self::Update(CounterArgs::try_from_slice(rest).unwrap()),
            3 => Self::Reset,
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}