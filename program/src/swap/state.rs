use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

#[derive(Debug, PartialEq)]
pub struct CreatorFee {
    pub rate: u32,
    pub balance_a: u64,
    pub balance_b: u64,
    pub withdraw_authority: Pubkey,
}

#[derive(Debug, PartialEq)]
pub struct SwapPool {
    pub seed: [u8; 32],
    pub token_account_a: Pubkey,
    pub token_account_b: Pubkey,
    pub balance_a: u64,
    pub balance_b: u64,
    pub lp_mint: Pubkey,
    pub lp_fee_rate: u32,
    pub creator_fee: Option<CreatorFee>,
}


impl SwapPool {
    pub const BASE_SIZE: usize = 1 + 32 + 32 + 32 + 8 + 8 + 32 + 4;
    pub const CREATOR_FEE_SIZE: usize = 4 + 8 + 8 + 32;
    pub const WITH_CREATOR_FEE_SIZE: usize = SwapPool::BASE_SIZE + SwapPool::CREATOR_FEE_SIZE;
    pub const TYPE_MARKER: u8 = 1;

    pub fn pack(&self, dst: &mut [u8]) -> Result<(), ProgramError> {
        if (self.creator_fee.is_none() && dst.len() != SwapPool::BASE_SIZE) ||
            (self.creator_fee.is_some() && dst.len() != SwapPool::WITH_CREATOR_FEE_SIZE) {
            return Err(ProgramError::InvalidAccountData);
        }

        let dst_ref = array_mut_ref![dst, 0, SwapPool::BASE_SIZE];
        let (type_marker_dst, seed_dst, token_acc_a_dst, token_acc_b_dst, balance_a_dst, balance_b_dst, lp_mint_dst, lp_fee_rate_dst)
            = mut_array_refs![dst_ref, 1, 32, 32, 32, 8, 8, 32, 4];

        *type_marker_dst = [SwapPool::TYPE_MARKER];
        seed_dst.copy_from_slice(self.seed.as_ref());
        token_acc_a_dst.copy_from_slice(self.token_account_a.as_ref());
        token_acc_b_dst.copy_from_slice(self.token_account_b.as_ref());
        *balance_a_dst = self.balance_a.to_le_bytes();
        *balance_b_dst = self.balance_b.to_le_bytes();
        lp_mint_dst.copy_from_slice(self.lp_mint.as_ref());
        *lp_fee_rate_dst = self.lp_fee_rate.to_le_bytes();

        if let Some(creator_fee) = &self.creator_fee {
            let dst_ref = array_mut_ref![dst, SwapPool::BASE_SIZE, SwapPool::CREATOR_FEE_SIZE];

            let (rate_dst, balance_a_dst, balance_b_dst, withdraw_authority_dst)
                = mut_array_refs![dst_ref, 4, 8, 8, 32];

            *rate_dst = creator_fee.rate.to_le_bytes();
            *balance_a_dst = creator_fee.balance_a.to_le_bytes();
            *balance_b_dst = creator_fee.balance_b.to_le_bytes();
            withdraw_authority_dst.copy_from_slice(creator_fee.withdraw_authority.as_ref());
        }

        Ok(())
    }

    pub fn unpack(src: &[u8]) -> Result<Self, ProgramError> {
        // todo: size check

        let src_array_ref = array_ref![src, 0, SwapPool::BASE_SIZE];
        let (type_marker, seed, token_acc_a, token_acc_b,
            balance_a, balance_b, lp_mint, lp_fee_rate)
            = array_refs![src_array_ref, 1, 32, 32, 32, 8, 8, 32, 4];

        if *type_marker != [SwapPool::TYPE_MARKER] {
            return Err(ProgramError::InvalidAccountData);
        }

        let creator_fee = if src.len() == SwapPool::WITH_CREATOR_FEE_SIZE {
            let src_array_ref = array_ref![src, SwapPool::BASE_SIZE, SwapPool::CREATOR_FEE_SIZE];
            let (rate, balance_a, balance_b, withdraw_authority)
                = array_refs![src_array_ref, 4, 8, 8, 32];

            Some(CreatorFee {
                rate: u32::from_le_bytes(*rate),
                balance_a: u64::from_le_bytes(*balance_a),
                balance_b: u64::from_le_bytes(*balance_b),
                withdraw_authority: Pubkey::new_from_array(*withdraw_authority),
            })
        } else {
            None
        };

        Ok(SwapPool {
            seed: *seed, // todo: should we clone ?
            token_account_a: Pubkey::new_from_array(*token_acc_a),
            token_account_b: Pubkey::new_from_array(*token_acc_b),
            balance_a: u64::from_le_bytes(*balance_a),
            balance_b: u64::from_le_bytes(*balance_b),
            lp_mint: Pubkey::new_from_array(*lp_mint),
            lp_fee_rate: u32::from_le_bytes(*lp_fee_rate),
            creator_fee,
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_pool_pack_unpack() {
        let pool_without_creator_fee = SwapPool {
            seed: Pubkey::new_unique().to_bytes(),
            token_account_a: Pubkey::new_unique(),
            token_account_b: Pubkey::new_unique(),
            balance_a: 100,
            balance_b: 150,
            lp_mint: Pubkey::new_unique(),
            lp_fee_rate: 5_000,
            creator_fee: None,
        };
        let mut state_array = [0u8; SwapPool::BASE_SIZE];
        pool_without_creator_fee.pack(&mut state_array).unwrap();
        assert_eq!(pool_without_creator_fee, SwapPool::unpack(&state_array).unwrap());
        assert!(pool_without_creator_fee.pack(&mut [0u8; SwapPool::WITH_CREATOR_FEE_SIZE]).is_err());


        let pool_with_creator_fee = SwapPool {
            seed: Pubkey::new_unique().to_bytes(),
            token_account_a: Pubkey::new_unique(),
            token_account_b: Pubkey::new_unique(),
            balance_a: 0,
            balance_b: 120,
            lp_mint: Pubkey::new_unique(),
            lp_fee_rate: 5_000,
            creator_fee: Some(CreatorFee {
                rate: 10_000,
                balance_a: 5_000,
                balance_b: 6_000,
                withdraw_authority: Default::default()
            }),
        };
        let mut state_array = [0u8; SwapPool::WITH_CREATOR_FEE_SIZE];
        pool_with_creator_fee.pack(&mut state_array).unwrap();
        assert_eq!(pool_with_creator_fee, SwapPool::unpack(&state_array).unwrap());
        assert!(pool_with_creator_fee.pack(&mut [0u8; SwapPool::BASE_SIZE]).is_err());
    }
}