use solana_program::account_info::{AccountInfo, next_account_info};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program::{invoke, invoke_signed};
use solana_program::pubkey::Pubkey;
use solana_program::program_pack::Pack;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use spl_token::state::{Account, Mint};
use solana_program::program_error::ProgramError::{IllegalOwner, InvalidAccountData, InvalidInstructionData, MissingRequiredSignature};
use crate::swap::state::SwapPool;
use crate::swap::instruction::{calculate_deposit_amounts, SwapInstruction};
use crate::processor::{create_spl_token_account, transfer_spl_token};

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    match SwapInstruction::unpack(instruction_data)? {
        SwapInstruction::CreatePool { seed } => {
            msg!("Swap:CreatePool");
            process_create_pool(program_id, accounts, seed)
        }
        SwapInstruction::Swap {} => {
            msg!("Swap:Swap");
            todo!()
        }
        SwapInstruction::Deposit { min_a, max_a, min_b, max_b } => {
            msg!("Swap:Deposit");
            process_deposit(program_id, accounts, min_a, max_a, min_b, max_b)
        }
        SwapInstruction::Withdraw {} => {
            msg!("Swap:Withdraw");
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
        6, // todo: decide
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


fn process_deposit(program_id: &Pubkey, accounts: &[AccountInfo], min_a: u64, max_a: u64, min_b: u64, max_b: u64)
                   -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let owner_info = next_account_info(accounts_iter)?;
    let swap_pool_state_info = next_account_info(accounts_iter)?;
    let source_a_info = next_account_info(accounts_iter)?;
    let destination_a_info = next_account_info(accounts_iter)?;
    let source_b_info = next_account_info(accounts_iter)?;
    let destination_b_info = next_account_info(accounts_iter)?;
    let lp_mint_info = next_account_info(accounts_iter)?;
    let destination_lp_info = next_account_info(accounts_iter)?;

    let spl_token_program = next_account_info(accounts_iter)?;

    if !owner_info.is_signer {
        return Err(MissingRequiredSignature);
    }

    if swap_pool_state_info.owner != program_id {
        return Err(IllegalOwner);
    }

    let swap_pool_state = SwapPool::unpack(&swap_pool_state_info.try_borrow_data()?)?;
    if swap_pool_state.token_account_a != *destination_a_info.key
        || swap_pool_state.token_account_b != *destination_b_info.key
        || swap_pool_state.lp_mint != *lp_mint_info.key {
        return Err(InvalidAccountData);
    }

    let lp_mint_state = Mint::unpack(&lp_mint_info.try_borrow_data()?)?;
    let destination_a_account_state = Account::unpack(&destination_a_info.try_borrow_data()?)?;
    let destination_b_account_state = Account::unpack(&destination_b_info.try_borrow_data()?)?;

    let (token_a_transfer_amount, token_b_transfer_amount, lp_mint_amount) = calculate_deposit_amounts(
        destination_a_account_state.amount,
        destination_b_account_state.amount,
        lp_mint_state.supply,
        max_a,
        max_b).ok_or(InvalidInstructionData)?;

    // todo: slippage check - fail if over it

    transfer_spl_token(
        source_a_info,
        destination_a_info,
        owner_info,
        spl_token_program,
        token_a_transfer_amount,
    )?;

    transfer_spl_token(
        source_b_info,
        destination_b_info,
        owner_info,
        spl_token_program,
        token_b_transfer_amount,
    )?;

    let mint_instruction = spl_token::instruction::mint_to(
        spl_token_program.key,
        &swap_pool_state.lp_mint,
        destination_lp_info.key,
        swap_pool_state_info.key,
        &[],
        lp_mint_amount,
    )?;

    invoke_signed(
        &mint_instruction,
        &[
            spl_token_program.clone(),
            lp_mint_info.clone(),
            destination_lp_info.clone(),
            swap_pool_state_info.clone(),
        ],
        &[&[&swap_pool_state.seed]],
    )?;

    Ok(())
}