use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

#[derive(Debug, PartialEq)]
pub struct SwapPool {
    pub seed: [u8; 32],
    pub token_account_a: Pubkey,
    pub token_account_b: Pubkey,
    pub lp_mint: Pubkey,

    // todo: fee rate
    // todo: dao fee account
}


impl SwapPool {
    pub const SIZE: usize = 1 + 32 + 32 + 32 + 32;
    pub const TYPE_MARKER: u8 = 1;

    pub fn pack(&self, dst: &mut [u8]) -> Result<(), ProgramError> {
        if dst.len() != SwapPool::SIZE {
            return Err(ProgramError::InvalidAccountData);
        }

        let dst_ref = array_mut_ref![dst, 0, SwapPool::SIZE];
        let (type_marker_dst, seed_dst, token_acc_a_dst, token_acc_b_dst, lp_mint_dst)
            = mut_array_refs![dst_ref, 1, 32, 32, 32, 32];

        *type_marker_dst = [SwapPool::TYPE_MARKER];
        seed_dst.copy_from_slice(self.seed.as_ref());
        token_acc_a_dst.copy_from_slice(self.token_account_a.as_ref());
        token_acc_b_dst.copy_from_slice(self.token_account_b.as_ref());
        lp_mint_dst.copy_from_slice(self.lp_mint.as_ref());

        Ok(())
    }

    pub fn unpack(src: &[u8]) -> Result<Self, ProgramError> {
        // todo: size check

        let src_array_ref = array_ref![src, 0, SwapPool::SIZE];
        let (type_marker, seed, token_acc_a, token_acc_b, lp_mint) = array_refs![src_array_ref, 1, 32, 32, 32, 32];

        if *type_marker != [SwapPool::TYPE_MARKER] {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(SwapPool {
            seed: *seed, // todo: should we clone ?
            token_account_a: Pubkey::new_from_array(*token_acc_a),
            token_account_b: Pubkey::new_from_array(*token_acc_b),
            lp_mint: Pubkey::new_from_array(*lp_mint),
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_pool_pack_unpack() {
        todo!()
    }
}