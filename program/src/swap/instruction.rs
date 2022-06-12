use solana_program::program_error::ProgramError;
use solana_program::program_error::ProgramError::InvalidInstructionData;

#[derive(Debug, PartialEq)]
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
    // 0. [signer] Fee payer, token accounts owner
    // 1. [] Swap pool state account - PDA
    // 2. [writeable] Source token A account
    // 3. [writeable] Destination token A account
    // 4. [writeable] Source token B account
    // 5. [writeable] Destination token B account
    // 6. [writeable] LP mint
    // 7. [writeable] Destination LP token account
    // 8. [] SPL token program
    Deposit {
        // todo: document properties
        min_a: u64,
        max_a: u64,
        min_b: u64,
        max_b: u64,
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
            SwapInstruction::Deposit { min_a, max_a, min_b, max_b } => {
                buffer.push(2);
                buffer.extend_from_slice(&min_a.to_le_bytes());
                buffer.extend_from_slice(&max_a.to_le_bytes());
                buffer.extend_from_slice(&min_b.to_le_bytes());
                buffer.extend_from_slice(&max_b.to_le_bytes());
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
            1 => {
                // swap
                todo!()
            }
            2 => {
                let min_a = rest.get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstructionData)?;

                let max_a = rest.get(8..16)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstructionData)?;

                let min_b = rest.get(16..24)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstructionData)?;

                let max_b = rest.get(24..32)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstructionData)?;

                Ok(SwapInstruction::Deposit { min_a, max_a, min_b, max_b })
            }
            _ => Err(InvalidInstructionData)
        }
    }
}


pub fn calculate_deposit_amounts(pool_a_amount: u64, pool_b_amount: u64, pool_lp_amount: u64,
                                 deposit_max_a: u64, deposit_max_b: u64) -> Option<(u64, u64, u64)> {
    if pool_lp_amount == 0 {
        // Deposit to empty pool
        return Some((deposit_max_a, deposit_max_b, 10_000_000_000 as u64));
    }

    let pool_ratio = (pool_a_amount as u128)
        .checked_mul(u64::MAX as u128)?
        .checked_div(pool_b_amount as u128)?;

    let deposit_ratio = (deposit_max_a as u128)
        .checked_mul(u64::MAX as u128)?
        .checked_div(deposit_max_b as u128)?;

    let (deposit_a, deposit_b) = if deposit_ratio >= pool_ratio {
        let deposit_a: u64 = (deposit_max_b as u128)
            .checked_mul(pool_ratio)?
            .checked_div(u64::MAX as u128)?
            .try_into().ok()?;

        (deposit_a, deposit_max_b)
    } else {
        let deposit_b : u64 = (deposit_max_a as u128)
            .checked_mul(u64::MAX as u128)?
            .checked_div(pool_ratio)?
            .try_into().ok()?;

        (deposit_max_a, deposit_b)
    };

    let lp_mint_amount = (deposit_a as u128)
        .checked_mul(u64::MAX as u128)?
        .checked_div(pool_a_amount as u128)?
        .checked_mul(pool_lp_amount as u128)?
        .checked_div(u64::MAX as u128)?
        .try_into().ok()?;

    Some((deposit_a, deposit_b, lp_mint_amount))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack_swap_instruction() {
        let instruction = SwapInstruction::Deposit {
            min_a: 1,
            max_a: 2,
            min_b: 3,
            max_b: 4,
        };

        assert_eq!(instruction, SwapInstruction::unpack(&instruction.pack()).unwrap());
        assert_ne!(instruction, SwapInstruction::unpack(&SwapInstruction::Deposit {
            min_a: 1,
            max_a: 1,
            min_b: 1,
            max_b: 1,
        }.pack()).unwrap());
    }


    #[test]
    fn test_calculate_deposit_amounts() {
        assert_eq!(
            Some((69, 420, 10_000_000_000)),
            calculate_deposit_amounts(0, 0, 0, 69, 420)
        );

        assert_eq!(
            Some((100, 100, 10_000)),
            calculate_deposit_amounts(100, 100, 10_000, 100, 100)
        );
        assert_eq!(
            Some((100, 100, 10_000)),
            calculate_deposit_amounts(100, 100, 10_000, 110, 100)
        );
        assert_eq!(
            Some((100, 100, 10_000)),
            calculate_deposit_amounts(100, 100, 10_000, 100, 110)
        );


        // todo: add more unit tests

        // todo: test input u64::MAX

        // todo: test for rounding error- printing money
    }
}