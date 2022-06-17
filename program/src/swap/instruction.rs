use solana_program::program_error::ProgramError;
use solana_program::program_error::ProgramError::InvalidInstructionData;

#[derive(Debug, PartialEq)]
pub enum SwapInstruction {
    // 1-0
    // Create swap pool
    // 0. [signer] Fee payer, swap pool creator
    // 1. [writeable] Swap pool state account - PDA
    // 2. [] Token A mint
    // 3. [writeable] Token A pool account
    // 4. [] Token B mint
    // 5. [writeable] Token B pool account
    // 6. [writeable] LP mint
    // 7. [] SPL token program
    // 8. [] System program
    CreatePool {
        seed: [u8; 32],
        lp_fee_rate: u32,
        creator_fee_rate: u32,
    },

    // 1-1
    // Swap tokens
    // 0. [signer] Fee payer, token accounts owner
    // 1. [writeable] Swap pool state account - PDA
    // 2. [writeable] Source input token account
    // 3. [writeable] Destination input token account
    // 4. [writeable] Source output token account
    // 5. [writeable] Destination output token account
    // 6. [] SPL token program
    // todo: add hodor config account - read dao fee rate from it
    Swap {
        in_amount: u64,
        min_out_amount: u64,
    },

    // 1-2
    // Deposit into pool
    // 0. [signer] Fee payer, token accounts owner
    // 1. [writeable] Swap pool state account - PDA
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

    // 1-3
    // Withdraw tokens from pool
    // 0. [signer] Fee payer, token accounts owner
    // 1. [writeable] Swap pool state account - PDA
    // 2. [writeable] Source token A account
    // 3. [writeable] Destination token A account
    // 4. [writeable] Source token B account
    // 5. [writeable] Destination token B account
    // 6. [writeable] LP mint
    // 7. [writeable] Source LP token account
    // 8. [] SPL token program
    Withdraw {
        lp_amount: u64,
        min_a: u64,
        min_b: u64,
    },

    // 1-4 ChangeCreatorWithdrawAuthority
    // 1-5 WithdrawCreatorFee
}

// todo: unit test pack/unpack swap instruction
impl SwapInstruction {
    const MODULE_TAG: u8 = 1;

    pub fn pack(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.push(SwapInstruction::MODULE_TAG);

        match self {
            SwapInstruction::CreatePool { seed, lp_fee_rate, creator_fee_rate } => {
                buffer.push(0);
                buffer.extend_from_slice(seed);
                buffer.extend_from_slice(&lp_fee_rate.to_le_bytes());
                buffer.extend_from_slice(&creator_fee_rate.to_le_bytes());
            }
            SwapInstruction::Swap { in_amount, min_out_amount } => {
                buffer.push(1);
                buffer.extend_from_slice(&in_amount.to_le_bytes());
                buffer.extend_from_slice(&min_out_amount.to_le_bytes());
            }
            SwapInstruction::Deposit { min_a, max_a, min_b, max_b } => {
                buffer.push(2);
                buffer.extend_from_slice(&min_a.to_le_bytes());
                buffer.extend_from_slice(&max_a.to_le_bytes());
                buffer.extend_from_slice(&min_b.to_le_bytes());
                buffer.extend_from_slice(&max_b.to_le_bytes());
            }
            SwapInstruction::Withdraw { lp_amount, min_a, min_b } => {
                buffer.push(3);
                buffer.extend_from_slice(&lp_amount.to_le_bytes());
                buffer.extend_from_slice(&min_a.to_le_bytes());
                buffer.extend_from_slice(&min_b.to_le_bytes())
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

                let lp_fee_rate = rest.get(32..36)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u32::from_le_bytes)
                    .ok_or(InvalidInstructionData)?;

                let creator_fee_rate = rest.get(36..40)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u32::from_le_bytes)
                    .ok_or(InvalidInstructionData)?;

                Ok(SwapInstruction::CreatePool { seed, lp_fee_rate, creator_fee_rate })
            }
            1 => {
                let in_amount = rest.get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstructionData)?;

                let min_out_amount = rest.get(8..16)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstructionData)?;

                Ok(SwapInstruction::Swap { in_amount, min_out_amount })
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
            3 => {
                let lp_amount = rest.get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstructionData)?;

                let min_a = rest.get(8..16)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstructionData)?;

                let min_b = rest.get(16..24)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstructionData)?;

                Ok(SwapInstruction::Withdraw { lp_amount, min_a, min_b })
            }
            _ => Err(InvalidInstructionData)
        }
    }
}


pub fn calculate_deposit_amounts(pool_a_amount: u64, pool_b_amount: u64, lp_supply: u64,
                                 deposit_max_a: u64, deposit_max_b: u64) -> Option<(u64, u64, u64)> {
    if lp_supply == 0 {
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
        let deposit_b: u64 = (deposit_max_a as u128)
            .checked_mul(u64::MAX as u128)?
            .checked_div(pool_ratio)?
            .try_into().ok()?;

        (deposit_max_a, deposit_b)
    };

    let lp_mint_amount = (deposit_a as u128)
        .checked_mul(u64::MAX as u128)?
        .checked_div(pool_a_amount as u128)?
        .checked_mul(lp_supply as u128)?
        .checked_div(u64::MAX as u128)?
        .try_into().ok()?;

    Some((deposit_a, deposit_b, lp_mint_amount))
}

const FEE_RATE_BASE_DIVIDER: u128 = 100_000_000;

fn calculate_fee_amount(amount: u128, fee_rate: u32) -> Option<u128> {
    Some(if fee_rate == 0 {
        0
    } else {
        amount
            .checked_mul(fee_rate as u128)?
            .checked_div(FEE_RATE_BASE_DIVIDER)?
    })
}

pub fn calculate_swap_amounts(pool_balance_in_token: u64, pool_balance_out_token: u64, swap_in_amount: u64,
                              dao_fee_rate: u32, lp_fee_rate: u32, creator_fee_rate: u32) -> Option<(u64, u64, u64, u64)> {
    let swap_in_amount = swap_in_amount as u128;

    let dao_fee_amount = calculate_fee_amount(swap_in_amount, dao_fee_rate)?;
    let lp_fee_amount = calculate_fee_amount(swap_in_amount, lp_fee_rate)?;
    let creator_fee_amount = calculate_fee_amount(swap_in_amount, creator_fee_rate)?;

    let pool_balance_in_token_after_fees = (pool_balance_in_token as u128)
        .checked_add(lp_fee_amount)?;
    let swap_in_amount_after_fees = swap_in_amount
        .checked_sub(dao_fee_amount)?
        .checked_sub(lp_fee_amount)?
        .checked_sub(creator_fee_amount)?;

    // x * y = k
    // (x + a)(y - b) = k
    // b = y * a / (x + a)
    let swap_out_amount = (pool_balance_out_token as u128)
        .checked_mul(swap_in_amount_after_fees)?
        .checked_div(
            pool_balance_in_token_after_fees
                .checked_add(swap_in_amount_after_fees)?
        )?;

    Some((
        swap_out_amount.try_into().ok()?,
        dao_fee_amount.try_into().ok()?,
        lp_fee_amount.try_into().ok()?,
        creator_fee_amount.try_into().ok()?
    ))
}

pub fn calculate_withdraw_amounts(pool_a_amount: u64, pool_b_amount: u64, lp_supply: u64,
                                  withdraw_lp_amount: u64) -> Option<(u64, u64)> {
    if withdraw_lp_amount == lp_supply {
        return Some((pool_a_amount, pool_b_amount));
    }

    let withdraw_ratio = (withdraw_lp_amount as u128)
        .checked_mul(u64::MAX as u128)?
        .checked_div(lp_supply as u128)?;

    let withdraw_a_amount = (pool_a_amount as u128)
        .checked_mul(withdraw_ratio)?
        .checked_div(u64::MAX as u128)?
        .try_into().ok()?;

    let withdraw_b_amount = (pool_b_amount as u128)
        .checked_mul(withdraw_ratio)?
        .checked_div(u64::MAX as u128)?
        .try_into().ok()?;

    Some((withdraw_a_amount, withdraw_b_amount))
}


#[cfg(test)]
mod tests {
    use solana_program::pubkey::Pubkey;
    use super::*;

    #[test]
    fn test_pack_unpack_swap_instruction() {
        let create_instruction = SwapInstruction::CreatePool {
            seed: Pubkey::new_unique().to_bytes(),
            lp_fee_rate: 5,
            creator_fee_rate: 60,
        };
        assert_eq!(create_instruction, SwapInstruction::unpack(&create_instruction.pack()).unwrap());
        assert_ne!(create_instruction, SwapInstruction::unpack(&SwapInstruction::CreatePool {
            seed: Default::default(),
            lp_fee_rate: 0,
            creator_fee_rate: 0,
        }.pack()).unwrap());


        let swap_instruction = SwapInstruction::Swap { in_amount: 1, min_out_amount: 2 };
        assert_eq!(swap_instruction, SwapInstruction::unpack(&swap_instruction.pack()).unwrap());
        assert_ne!(swap_instruction, SwapInstruction::unpack(&SwapInstruction::Swap {
            in_amount: 0,
            min_out_amount: 0,
        }.pack()).unwrap());

        let deposit_instruction = SwapInstruction::Deposit {
            min_a: 1,
            max_a: 2,
            min_b: 3,
            max_b: 4,
        };

        assert_eq!(deposit_instruction, SwapInstruction::unpack(&deposit_instruction.pack()).unwrap());
        assert_ne!(deposit_instruction, SwapInstruction::unpack(&SwapInstruction::Deposit {
            min_a: 1,
            max_a: 1,
            min_b: 1,
            max_b: 1,
        }.pack()).unwrap());


        let withdraw_instruction = SwapInstruction::Withdraw {
            lp_amount: 1,
            min_a: 2,
            min_b: 3,
        };
        assert_eq!(withdraw_instruction, SwapInstruction::unpack(&withdraw_instruction.pack()).unwrap());
        assert_ne!(withdraw_instruction, SwapInstruction::unpack(&SwapInstruction::Withdraw {
            lp_amount: 1,
            min_a: 1,
            min_b: 1,
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

    #[test]
    fn test_calculate_swap_amounts_with_fees() {
        // 1% for every fee type
        assert_eq!(
            Some((88_342, 1_000, 1_000, 1_000)),
            calculate_swap_amounts(1_000_000, 1_000_000, 100_000,
                                   1_000_000, 1_000_000, 1_000_000)
        );

        // 90.99% fee
        assert_eq!(
            Some((8920, 90000, 990, 0)),
            calculate_swap_amounts(1_000_000, 1_000_000, 100_000,
                                   90_000_000, 990_000, 0)
        );

        // 90.99% fee
        assert_eq!(
            Some((8198, 990, 90000, 0)),
            calculate_swap_amounts(1_000_000, 1_000_000, 100_000,
                                   990_000, 90_000_000, 0)
        );

        // over 100% total fee
        assert_eq!(
            None,
            calculate_swap_amounts(1_000_000, 1_000_000, 100_000,
                                   50_000_000, 50_000_000, 1_000_000)
        );

        // todo: add more unit tests
    }

    #[test]
    fn test_calculate_swap_amounts_without_fees() {
        assert_eq!(Some((0, 0, 0, 0)), calculate_swap_amounts(1, 100, 0, 0, 0, 0));
        assert_eq!(Some((0, 0, 0, 0)), calculate_swap_amounts(100, 10, 11, 0, 0, 0));
        assert_eq!(Some((1, 0, 0, 0)), calculate_swap_amounts(100_000_000, 100, 1_011_000, 0, 0, 0));
        assert_eq!(Some((4, 0, 0, 0)), calculate_swap_amounts(100, 100, 5, 0, 0, 0));
        assert_eq!(Some((49_950_049, 0, 0, 0)), calculate_swap_amounts(100_000_000_000, 50_000_000_000, 100_000_000, 0, 0, 0));
        assert_eq!(Some((372_208_436, 0, 0, 0)), calculate_swap_amounts(100_000_000_000, 50_000_000_000, 750_000_000, 0, 0, 0));
        assert_eq!(Some((3_333_333, 0, 0, 0)), calculate_swap_amounts(10_000_000, 10_000_000, 5_000_000, 0, 0, 0));
        assert_eq!(Some((6_666_666, 0, 0, 0)), calculate_swap_amounts(10_000_000, 10_000_000, 20_000_000, 0, 0, 0));
        assert_eq!(Some((8_000_000, 0, 0, 0)), calculate_swap_amounts(10_000_000, 10_000_000, 40_000_000, 0, 0, 0));
        assert_eq!(Some((12_990_906, 0, 0, 0)), calculate_swap_amounts(70_000_000, 13_000_000, 100_000_000_000, 0, 0, 0));
    }

    #[test]
    fn test_calculate_withdraw_amounts() {
        assert_eq!(
            Some((10, 10)),
            calculate_withdraw_amounts(10, 10, 100, 100)
        );

        assert_eq!(
            Some((4_999, 4_999)),
            calculate_withdraw_amounts(10_000, 10_000, 100_000, 50_000)
        );

        assert_eq!(
            Some((4_999_999_999, 4_999_999_999)),
            calculate_withdraw_amounts(10_000_000_000, 10_000_000_000, 100_000_000_000, 50_000_000_000)
        );

        assert_eq!(
            Some((5_000, 5_000)),
            calculate_withdraw_amounts(10_000, 10_000, 100_000, 50_001)
        );

        assert_eq!(
            Some((5_000, 2_500)),
            calculate_withdraw_amounts(10_000, 5_000, 100_000, 50_001)
        );

        assert_eq!(
            Some((2_500, 5_000)),
            calculate_withdraw_amounts(5_000, 10_000, 100_000, 50_001)
        );

        assert_eq!(
            Some((0, 0)),
            calculate_withdraw_amounts(5_000, 10_000, 100_000, 1)
        );

        assert_eq!(
            Some((49, 29)),
            calculate_withdraw_amounts(5_000_000, 3_000_000, 100_000, 1)
        );

        assert_eq!(
            Some((0, 0)),
            calculate_withdraw_amounts(10, 10, 100, 9)
        );

        // todo: tests with rounding errors
        // todo: tests with overflow
    }
}