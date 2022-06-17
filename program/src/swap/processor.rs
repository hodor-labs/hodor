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
use crate::swap::state::{CreatorFee, SwapPool};
use crate::swap::instruction::{calculate_deposit_amounts, calculate_swap_amounts, calculate_withdraw_amounts, SwapInstruction};
use crate::processor::{create_spl_token_account, transfer_spl_token};

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    match SwapInstruction::unpack(instruction_data)? {
        SwapInstruction::CreatePool { seed, lp_fee_rate, creator_fee_rate } => {
            msg!("Swap:CreatePool");
            process_create_pool(program_id, accounts, seed, lp_fee_rate, creator_fee_rate)
        }
        SwapInstruction::Swap { in_amount, min_out_amount } => {
            msg!("Swap:Swap");
            process_swap(program_id, accounts, in_amount, min_out_amount)
        }
        SwapInstruction::Deposit { min_a, max_a, min_b, max_b } => {
            msg!("Swap:Deposit");
            process_deposit(program_id, accounts, min_a, max_a, min_b, max_b)
        }
        SwapInstruction::Withdraw { lp_amount, min_a, min_b } => {
            msg!("Swap:Withdraw");
            process_wthdraw(program_id, accounts, lp_amount, min_a, min_b)
        }
    }
}

fn process_create_pool(program_id: &Pubkey, accounts: &[AccountInfo], seed: [u8; 32],
                       lp_fee_rate: u32, creator_fee_rate: u32) -> ProgramResult {
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

    if !fee_payer_info.is_signer {
        return Err(MissingRequiredSignature);
    }

    if token_a_mint_info.key == token_b_mint_info.key {
        return Err(InvalidAccountData);
    }

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
        None,
        6,
    )?;

    invoke(
        &initialize_mint_instruction,
        &[
            spl_token_program.clone(),
            lp_mint_info.clone(),
        ],
    )?;

    let (creator_fee, state_size) = {
        if creator_fee_rate > 0 {
            (Some(CreatorFee {
                rate: creator_fee_rate,
                balance_a: 0,
                balance_b: 0,
                withdraw_authority: fee_payer_info.key.clone(),
            }), SwapPool::WITH_CREATOR_FEE_SIZE)
        } else {
            (None, SwapPool::BASE_SIZE)
        }
    };


    let create_state_account_instruction = solana_program::system_instruction::create_account(
        &fee_payer_info.key,
        &swap_state_info.key,
        rent.minimum_balance(state_size),
        state_size as u64,
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
        balance_a: 0,
        balance_b: 0,
        lp_mint: lp_mint_account,
        lp_fee_rate,
        creator_fee: creator_fee,
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

    let mut swap_pool_state = SwapPool::unpack(&swap_pool_state_info.try_borrow_data()?)?;
    if swap_pool_state.token_account_a != *destination_a_info.key
        || swap_pool_state.token_account_b != *destination_b_info.key
        || swap_pool_state.lp_mint != *lp_mint_info.key {
        return Err(InvalidAccountData);
    }

    let lp_mint_state = Mint::unpack(&lp_mint_info.try_borrow_data()?)?;

    let (token_a_transfer_amount, token_b_transfer_amount, lp_mint_amount) = calculate_deposit_amounts(
        swap_pool_state.balance_a,
        swap_pool_state.balance_b,
        lp_mint_state.supply,
        max_a,
        max_b).ok_or(InvalidInstructionData)?;

    if token_a_transfer_amount < min_a || token_b_transfer_amount < min_b {
        // todo: add custom error message
        return Err(InvalidInstructionData);
    }

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


    swap_pool_state.balance_a = swap_pool_state.balance_a
        .checked_add(token_a_transfer_amount)
        .ok_or(InvalidInstructionData)?;
    swap_pool_state.balance_b = swap_pool_state.balance_b
        .checked_add(token_b_transfer_amount)
        .ok_or(InvalidInstructionData)?;
    swap_pool_state.pack(&mut swap_pool_state_info.try_borrow_mut_data()?)?;

    Ok(())
}

fn process_swap(program_id: &Pubkey, accounts: &[AccountInfo], in_amount: u64, min_out_amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let owner_info = next_account_info(accounts_iter)?;
    let swap_pool_state_info = next_account_info(accounts_iter)?;
    let input_source_info = next_account_info(accounts_iter)?;
    let input_destination_info = next_account_info(accounts_iter)?;
    let output_source_info = next_account_info(accounts_iter)?;
    let output_destination_info = next_account_info(accounts_iter)?;

    let spl_token_program = next_account_info(accounts_iter)?;

    if !owner_info.is_signer {
        return Err(MissingRequiredSignature);
    }

    if swap_pool_state_info.owner != program_id {
        return Err(IllegalOwner);
    }

    let mut swap_pool_state = SwapPool::unpack(&swap_pool_state_info.try_borrow_data()?)?;

    // todo: this conditions need to be unit tested
    let is_a_to_b = {
        if *input_destination_info.key == swap_pool_state.token_account_a
            && *output_source_info.key == swap_pool_state.token_account_b {
            true
        } else if *input_destination_info.key == swap_pool_state.token_account_b
            && *output_source_info.key == swap_pool_state.token_account_a {
            false
        } else {
            return Err(InvalidAccountData);
        }
    };

    let input_destination_state = Account::unpack(&input_destination_info.try_borrow_data()?)?;
    let output_source_state = Account::unpack(&output_source_info.try_borrow_data()?)?;

    let (pool_balance_in_token, pool_balance_out_token) = if is_a_to_b {
        (swap_pool_state.balance_a, swap_pool_state.balance_b)
    } else {
        (swap_pool_state.balance_b, swap_pool_state.balance_a)
    };

    let (out_amount, dao_fee_amount, lp_fee_amount, creator_fee_amount) = calculate_swap_amounts(
        pool_balance_in_token,
        pool_balance_out_token,
        in_amount,
        50_000, // hardcoded 0.05%, todo: read from dao controlled config account
        swap_pool_state.lp_fee_rate,
        swap_pool_state.creator_fee.as_ref()
            .map_or(0, |cf| cf.rate),
    ).ok_or(InvalidInstructionData)?;

    if out_amount < min_out_amount {
        // todo: add custom error message
        return Err(InvalidInstructionData);
    }

    transfer_spl_token(
        input_source_info,
        input_destination_info,
        owner_info,
        spl_token_program,
        in_amount,
    )?;

    let withdraw_transfer_instruction = spl_token::instruction::transfer(
        spl_token_program.key,
        output_source_info.key,
        output_destination_info.key,
        swap_pool_state_info.key,
        &[],
        out_amount,
    )?;
    invoke_signed(
        &withdraw_transfer_instruction,
        &[
            spl_token_program.clone(),
            output_source_info.clone(),
            output_destination_info.clone(),
            swap_pool_state_info.clone(),
        ],
        &[&[&swap_pool_state.seed]],
    )?;

    let pool_deposit_amount = in_amount
        .checked_sub(dao_fee_amount)
        .ok_or(InvalidInstructionData)?
        .checked_sub(creator_fee_amount)
        .ok_or(InvalidInstructionData)?;

    if is_a_to_b {
        swap_pool_state.balance_a = swap_pool_state.balance_a
            .checked_add(pool_deposit_amount)
            .ok_or(InvalidInstructionData)?;
        swap_pool_state.balance_b = swap_pool_state.balance_b
            .checked_sub(out_amount)
            .ok_or(InvalidInstructionData)?;
    } else {
        swap_pool_state.balance_b = swap_pool_state.balance_b
            .checked_add(pool_deposit_amount)
            .ok_or(InvalidInstructionData)?;
        swap_pool_state.balance_a = swap_pool_state.balance_a
            .checked_sub(out_amount)
            .ok_or(InvalidInstructionData)?;
    }

    if let Some(creator_fee) = &mut swap_pool_state.creator_fee {
        if is_a_to_b {
            creator_fee.balance_a = creator_fee.balance_a.checked_add(creator_fee_amount)
                .ok_or(InvalidInstructionData)?;
        } else {
            creator_fee.balance_b = creator_fee.balance_b.checked_add(creator_fee_amount)
                .ok_or(InvalidInstructionData)?;
        }
    }
    swap_pool_state.pack(&mut swap_pool_state_info.try_borrow_mut_data()?)?;

    Ok(())
}

fn process_wthdraw(program_id: &Pubkey, accounts: &[AccountInfo], lp_amount: u64, min_a: u64, min_b: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let owner_info = next_account_info(accounts_iter)?;
    let swap_pool_state_info = next_account_info(accounts_iter)?;
    let source_a_info = next_account_info(accounts_iter)?;
    let destination_a_info = next_account_info(accounts_iter)?;
    let source_b_info = next_account_info(accounts_iter)?;
    let destination_b_info = next_account_info(accounts_iter)?;
    let lp_mint_info = next_account_info(accounts_iter)?;
    let source_lp_info = next_account_info(accounts_iter)?;

    let spl_token_program = next_account_info(accounts_iter)?;

    if !owner_info.is_signer {
        return Err(MissingRequiredSignature);
    }

    if swap_pool_state_info.owner != program_id {
        return Err(IllegalOwner);
    }

    let mut swap_pool_state = SwapPool::unpack(&swap_pool_state_info.try_borrow_data()?)?;
    if swap_pool_state.token_account_a != *source_a_info.key
        || swap_pool_state.token_account_b != *source_b_info.key
        || swap_pool_state.lp_mint != *lp_mint_info.key {
        return Err(InvalidAccountData);
    }

    let lp_mint_state = Mint::unpack(&lp_mint_info.try_borrow_data()?)?;

    let (withdraw_a_amount, withdraw_b_amount) = calculate_withdraw_amounts(
        swap_pool_state.balance_a,
        swap_pool_state.balance_b,
        lp_mint_state.supply,
        lp_amount,
    ).ok_or(InvalidInstructionData)?;

    if withdraw_a_amount < min_a || withdraw_b_amount < min_b {
        // todo: add custom error message
        return Err(InvalidInstructionData);
    }

    // todo: test burning of more than provided account have
    let burn_instruction = spl_token::instruction::burn(
        spl_token_program.key,
        source_lp_info.key,
        &swap_pool_state.lp_mint,
        owner_info.key,
        &[owner_info.key],
        lp_amount,
    )?;

    invoke(
        &burn_instruction,
        &[
            spl_token_program.clone(),
            source_lp_info.clone(),
            lp_mint_info.clone(),
            owner_info.clone(),
        ],
    )?;

    let transfer_a_instruction = spl_token::instruction::transfer(
        spl_token_program.key,
        &swap_pool_state.token_account_a,
        destination_a_info.key,
        swap_pool_state_info.key,
        &[],
        withdraw_a_amount,
    )?;
    invoke_signed(
        &transfer_a_instruction,
        &[
            spl_token_program.clone(),
            source_a_info.clone(),
            destination_a_info.clone(),
            swap_pool_state_info.clone(),
        ],
        &[&[&swap_pool_state.seed]],
    )?;

    let transfer_b_instruction = spl_token::instruction::transfer(
        spl_token_program.key,
        &swap_pool_state.token_account_b,
        destination_b_info.key,
        swap_pool_state_info.key,
        &[],
        withdraw_b_amount,
    )?;
    invoke_signed(
        &transfer_b_instruction,
        &[
            spl_token_program.clone(),
            source_b_info.clone(),
            destination_b_info.clone(),
            swap_pool_state_info.clone(),
        ],
        &[&[&swap_pool_state.seed]],
    )?;

    swap_pool_state.balance_a = swap_pool_state.balance_a
        .checked_sub(withdraw_a_amount)
        .ok_or(InvalidInstructionData)?;
    swap_pool_state.balance_b = swap_pool_state.balance_b
        .checked_sub(withdraw_b_amount)
        .ok_or(InvalidInstructionData)?;
    swap_pool_state.pack(&mut swap_pool_state_info.try_borrow_mut_data()?)?;

    Ok(())
}