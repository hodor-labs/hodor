use solana_program::account_info::{AccountInfo, next_account_info};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program::{invoke, invoke_signed};
use solana_program::pubkey::Pubkey;
use crate::swap::instruction::SwapInstruction;
use crate::processor::create_spl_token_account;
use solana_program::program_pack::Pack;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use crate::swap::state::SwapPool;

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    match SwapInstruction::unpack(instruction_data)? {
        SwapInstruction::CreatePool { seed } => {
            msg!("Swap:CreatePool");
            process_create_pool(program_id, accounts, seed)
        }
        SwapInstruction::Deposit { } => {
            msg!("Swap:Deposit");
            todo!()
        }
    }
}

fn process_create_pool(program_id: &Pubkey, accounts: &[AccountInfo], seed: [u8; 32]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let fee_payer_info = next_account_info(accounts_iter)?;
    let swap_state_info = next_account_info(accounts_iter)?;
    let token_a_mint_info = next_account_info(accounts_iter)?;
    let token_a_account_info = next_account_info(accounts_iter)?;
    let token_b_mint_info = next_account_info(accounts_iter)?;
    let token_b_account_info = next_account_info(accounts_iter)?;
    let lp_mint_info = next_account_info(accounts_iter)?;

    let spl_token_program = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let rent = Rent::get()?;

    let seeds_a = [swap_state_info.key.as_ref(), b"A"];
    create_spl_token_account(
        token_a_account_info,
        token_a_mint_info,
        swap_state_info,
        fee_payer_info,
        &seeds_a,
        program_id,
        spl_token_program,
        system_program,
    )?;

    let seeds_b = [swap_state_info.key.as_ref(), b"B"];
    create_spl_token_account(
        token_b_account_info,
        token_b_mint_info,
        swap_state_info,
        fee_payer_info,
        &seeds_b,
        program_id,
        spl_token_program,
        system_program,
    )?;

    // Creating new mint for LP token
    let seeds_mint = [swap_state_info.key.as_ref(), b"LP"];
    let (lp_mint_account, bump_seed) = Pubkey::find_program_address(&seeds_mint, program_id);

    let create_mint_account_instruction = solana_program::system_instruction::create_account(
        &fee_payer_info.key,
        &lp_mint_account,
        rent.minimum_balance(spl_token::state::Mint::LEN),
        spl_token::state::Mint::LEN as u64,
        &spl_token::id(),
    );
    invoke_signed(
        &create_mint_account_instruction,
        &[
            system_program.clone(),
            fee_payer_info.clone(),
            lp_mint_info.clone(),
        ],
        &[&[&seeds_mint[0], &seeds_mint[1], &[bump_seed]]], // todo seeds explode
    )?;

    let initialize_mint_instruction = spl_token::instruction::initialize_mint2(
        &spl_token_program.key,
        &lp_mint_account,
        &swap_state_info.key,
        None, // todo: consider giving it to pool account (controllable later through DAO)
        9, // todo: decide
    )?;

    invoke(
        &initialize_mint_instruction,
        &[
            spl_token_program.clone(),
            lp_mint_info.clone(),
        ],
    )?;

    // For now this account is just marker, todo: put state
    let create_state_account_instruction = solana_program::system_instruction::create_account(
        &fee_payer_info.key,
        &swap_state_info.key,
        rent.minimum_balance(SwapPool::SIZE),
        SwapPool::SIZE as u64,
        &program_id,
    );

    // todo: test making sure it fails if account exists or if seed is incorrect
    invoke_signed(
        &create_state_account_instruction,
        &[
            system_program.clone(),
            fee_payer_info.clone(),
            swap_state_info.clone(),
        ],
        &[&[&seed]],
    )?;

    SwapPool {
        seed: seed,
        token_account_a: *token_a_account_info.key,
        token_account_b: *token_b_account_info.key,
        lp_mint: lp_mint_account,
    }.pack(&mut swap_state_info.try_borrow_mut_data()?)?;

    Ok(())
}