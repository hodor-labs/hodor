use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError::InvalidInstructionData;
use solana_program::pubkey::Pubkey;
use solana_program::program_pack::Pack;
use solana_program::program::{invoke, invoke_signed};
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use crate::swap;

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    let module_tag = instruction_data.first().ok_or(InvalidInstructionData)?;

    match module_tag {
        1 => {
            swap::processor::process(program_id, accounts, instruction_data)
        },
        _ => Err(InvalidInstructionData),
    }
}

pub(crate) fn create_spl_token_account<'a>(
    account: &AccountInfo<'a>, mint: &AccountInfo<'a>, owner: &AccountInfo<'a>, fee_payer: &AccountInfo<'a>, seeds: &[&[u8]],
    program_id: &Pubkey, spl_token_program: &AccountInfo<'a>, system_program: &AccountInfo<'a>
) -> ProgramResult {
    let rent = Rent::get()?;
    let (token_account, bump_seed) = Pubkey::find_program_address(&seeds, program_id);
    // todo: should we check account.key == token_account ? Below instructions should fail if it's not true

    let create_account_instruction = solana_program::system_instruction::create_account(
        &fee_payer.key,
        &token_account,
        rent.minimum_balance(spl_token::state::Account::LEN),
        spl_token::state::Account::LEN as u64,
        &spl_token::id(),
    );

    invoke_signed(
        &create_account_instruction,
        &[
            system_program.clone(),
            fee_payer.clone(),
            account.clone(),
        ],
        &[&[&seeds[0], &seeds[1], &[bump_seed]]], // todo seeds explode
    )?;

    let spl_initialize_instruction = spl_token::instruction::initialize_account3(
        &spl_token_program.key,
        &token_account,
        &mint.key,
        &owner.key,
    )?;

    invoke(
        &spl_initialize_instruction,
        &[
            spl_token_program.clone(),
            account.clone(),
            mint.clone(),
        ],
    )?;

    Ok(())
}