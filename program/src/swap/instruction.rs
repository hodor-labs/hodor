use solana_program::program_error::ProgramError;
use solana_program::program_error::ProgramError::InvalidInstructionData;

pub enum SwapInstruction {
    // 10
    // Create swap pool
    // 0. [signer] Fee payer
    // 1. [writeable] Swap pool state account - PDA
    // 2. [] Token A mint
    // 3. [writeable] Token A pool account
    // 4. [] Token B mint
    // 5. [writeable] Token B pool account
    // 6. [writeable] LP mint
    // todo: fee account
    // 7. [] SPL token program
    // 8. [] System program
    CreatePool {
        seed: [u8; 32],
        // todo feeRate - %
    },

    // 11
    Swap {
        // todo: slippage
    },

    // 12
    // Deposit into pool
    Deposit {
        // todo: slippage control
    },

    // 13
    // Withdraw
    Withdraw {
        // todo: slippage control ?
    },
}

// todo: unit test pack/unpack swap instruction
impl SwapInstruction {
    const MODULE_TAG: u8 = 1;

    pub fn pack(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.push(SwapInstruction::MODULE_TAG);

        match self {
            SwapInstruction::CreatePool { seed } => {
                buffer.push(0);
                buffer.extend_from_slice(seed);
            }
            SwapInstruction::Swap {} => {
                todo!()
            }
            SwapInstruction::Deposit {} => {
                todo!()
            }
            SwapInstruction::Withdraw {} => {
                todo!()
            }
        };

        buffer
    }

    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (module_tag, rest) = input.split_first().ok_or(InvalidInstructionData)?;
        if *module_tag != SwapInstruction::MODULE_TAG {
            return Err(InvalidInstructionData);
        }

        let (tag, rest) = rest.split_first().ok_or(InvalidInstructionData)?;

        match tag {
            0 => {
                let seed = rest
                    .get(..32)
                    .and_then(|slice| slice.try_into().ok())
                    .ok_or(InvalidInstructionData)?;

                Ok(SwapInstruction::CreatePool { seed })
            }
            2 => {
                // Deposit
                todo!()
            }
            _ => Err(InvalidInstructionData)
        }
    }
}