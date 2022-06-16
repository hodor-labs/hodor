use std::str::FromStr;
use clap::ArgMatches;
use solana_account_decoder::parse_token::UiTokenAccount;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_sdk::account::ReadableAccount;
use solana_sdk::signature::{Keypair, read_keypair_file};
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use spl_token::{amount_to_ui_amount, ui_amount_to_amount};
use hodor_program::swap::instruction::SwapInstruction;
use hodor_program::swap::state::SwapPool;
use crate::{Context, Error};

pub fn create_pool(context: Context, matches: &ArgMatches) -> Result<(), Error> {
    let mint_a = Pubkey::from_str(matches.value_of("TOKEN-A").unwrap())
        .map_err(|_| format!("Invalid token address"))?;

    let mint_b = Pubkey::from_str(matches.value_of("TOKEN-B").unwrap())
        .map_err(|_| format!("Invalid token address"))?;

    // todo: should be part of context
    let payer_keypair = read_keypair_file(context.cli_config.keypair_path)?;

    let (seed, state_account) = {
        let mut random_pubkey = Keypair::new().pubkey();
        let mut program_address;
        loop {
            program_address = Pubkey::create_program_address(
                &[random_pubkey.to_bytes().as_ref()], &context.program_id);
            if program_address.is_ok() {
                break;
            }
            random_pubkey = Keypair::new().pubkey();
        }
        (random_pubkey.to_bytes(), program_address.unwrap())
    };

    println!("Seed: {}", Pubkey::new(&seed));
    println!("State account: {}", state_account);

    println!("Mint A: {}", mint_a);
    println!("Mint B: {}", mint_b);

    let seed_a = [state_account.as_ref(), b"A"];
    let (token_a_account, _) = Pubkey::find_program_address(&seed_a, &context.program_id);
    println!("Token A account: {}", token_a_account);

    let seed_b = [state_account.as_ref(), b"B"];
    let (token_b_account, _) = Pubkey::find_program_address(&seed_b, &context.program_id);
    println!("Token B account: {}", token_b_account);

    let seeds_mint = [state_account.as_ref(), b"LP"];// todo: make it helper method in program ?
    let (lp_mint_account, _) = Pubkey::find_program_address(&seeds_mint, &context.program_id);

    let create_swap_pool_instruction = Instruction::new_with_bytes(
        context.program_id,
        &SwapInstruction::pack(&SwapInstruction::CreatePool { seed }),
        vec![
            AccountMeta::new(payer_keypair.pubkey(), true),
            AccountMeta::new(state_account, false),
            AccountMeta::new_readonly(mint_a, false),
            AccountMeta::new(token_a_account, false),
            AccountMeta::new_readonly(mint_b, false),
            AccountMeta::new(token_b_account, false),
            AccountMeta::new(lp_mint_account, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
    );

    let transaction = Transaction::new_signed_with_payer(
        &[create_swap_pool_instruction],
        Some(&payer_keypair.pubkey()),
        &[&payer_keypair],
        context.rpc_client.get_latest_blockhash()?,
    );

    let transaction_result = context.rpc_client.send_and_confirm_transaction(&transaction);
    println!("Transaction {:?}", transaction_result);

    Ok(())
}

pub fn deposit(context: Context, matches: &ArgMatches) -> Result<(), Error> {
    let pool_key = Pubkey::from_str(matches.value_of("POOL-ACCOUNT").unwrap())
        .map_err(|_| format!("Invalid swap pool account"))?;

    // todo: should be part of context
    let payer_keypair = read_keypair_file(&context.cli_config.keypair_path)?;

    let (pool_state, pool_account_a, pool_account_b)
        = get_pool_state_and_token_accounts(&context, &pool_key)?;

    let mint_a = Pubkey::from_str(&pool_account_a.mint)?;

    let amount_a = spl_token::ui_amount_to_amount(
        f64::from_str(matches.value_of("AMOUNT-A").unwrap())?,
        pool_account_a.token_amount.decimals);

    // todo: possibility to override through CLI param
    let source_account_a_key = get_associated_token_address(&payer_keypair.pubkey(), &mint_a);

    // todo: read account A state & check if enough balance

    let mint_b = Pubkey::from_str(&pool_account_b.mint)?;

    let amount_b = spl_token::ui_amount_to_amount(
        f64::from_str(matches.value_of("AMOUNT-B").unwrap())?,
        pool_account_b.token_amount.decimals);

    // todo: possibility to override through CLI param
    let source_account_b_key = get_associated_token_address(&payer_keypair.pubkey(), &mint_b);

    // todo: read account B state & check if enough balance

    let mut instructions = Vec::new();

    // todo: configurable through param
    let lp_destination = get_associated_token_address(&payer_keypair.pubkey(), &pool_state.lp_mint);
    if context.rpc_client.get_token_account(&lp_destination).is_err() {
        instructions.push(spl_associated_token_account::instruction::create_associated_token_account(
            &payer_keypair.pubkey(),
            &payer_keypair.pubkey(),
            &pool_state.lp_mint,
        ));
    }

    // todo: slippage control through CLI, for now hardcoded 1%
    let min_a = amount_a - (amount_a / 100);
    let min_b = amount_b - (amount_b / 100);

    instructions.push(Instruction::new_with_bytes(
        context.program_id,
        &SwapInstruction::pack(&SwapInstruction::Deposit { min_a, max_a: amount_a, min_b, max_b: amount_b }),
        vec![
            AccountMeta::new(payer_keypair.pubkey(), true),
            AccountMeta::new_readonly(pool_key, false),
            AccountMeta::new(source_account_a_key, false),
            AccountMeta::new(pool_state.token_account_a, false),
            AccountMeta::new(source_account_b_key, false),
            AccountMeta::new(pool_state.token_account_b, false),
            AccountMeta::new(pool_state.lp_mint, false),
            AccountMeta::new(lp_destination, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
    ));

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer_keypair.pubkey()),
        &[&payer_keypair],
        context.rpc_client.get_latest_blockhash()?,
    );

    let transaction_result = context.rpc_client.send_and_confirm_transaction(&transaction);
    println!("Transaction {:?}", transaction_result);

    Ok(())
}

pub fn print_info(context: Context, matches: &ArgMatches) -> Result<(), Error> {
    let pool_key = Pubkey::from_str(matches.value_of("POOL-ACCOUNT").unwrap())
        .map_err(|_| format!("Invalid swap pool account"))?;

    let (pool_state, token_acc_a, token_acc_b)
        = get_pool_state_and_token_accounts(&context, &pool_key)?;

    println!("Token A:");
    println!("MINT: {}", token_acc_a.mint);
    println!("Account: {}", pool_state.token_account_a);
    println!("Balance: {}", token_acc_a.token_amount.ui_amount_string);
    println!();
    println!("Token B:");
    println!("MINT: {}", token_acc_b.mint);
    println!("Account: {}", pool_state.token_account_b);
    println!("Balance: {}", token_acc_b.token_amount.ui_amount_string);
    println!();
    println!("LP MINT: {}", pool_state.lp_mint);
    /* todo
    println!("Fee: 0%");
    println!("Ratio: 1/0.34 == 0.34/1");*/

    Ok(())
}

pub fn swap(context: Context, matches: &ArgMatches) -> Result<(), Error> {
    let pool_key = Pubkey::from_str(matches.value_of("POOL-ACCOUNT").unwrap())
        .map_err(|_| format!("Invalid swap pool account"))?;

    // todo: should be part of context
    let payer_keypair = read_keypair_file(&context.cli_config.keypair_path)?;

    let (pool_state, pool_acc_a, pool_acc_b)
        = get_pool_state_and_token_accounts(&context, &pool_key)?;

    let pool_mint_a = Pubkey::from_str(&pool_acc_a.mint)?;
    let pool_mint_b = Pubkey::from_str(&pool_acc_b.mint)?;

    let input_account_key = Pubkey::from_str(matches.value_of("INPUT-ACCOUNT").unwrap())
        .map_err(|_| format!("Invalid input account"))?;

    let (in_source_key, in_destination_key, in_destination_acc) = {
        if input_account_key == pool_mint_a {
            (
                get_associated_token_address(&payer_keypair.pubkey(), &pool_mint_a),
                pool_state.token_account_a,
                &pool_acc_a
            )
        } else if input_account_key == pool_mint_b {
            (
                get_associated_token_address(&payer_keypair.pubkey(), &pool_mint_b),
                pool_state.token_account_b,
                &pool_acc_b
            )
        } else {
            let input_account = context.rpc_client.get_token_account_with_commitment(
                &input_account_key, context.commitment,
            )?.value.ok_or(format!("Provided token account: {} doesn't exist", input_account_key))?;

            let mint = Pubkey::from_str(&input_account.mint)?;

            if mint == pool_mint_a {
                (input_account_key, pool_state.token_account_a, &pool_acc_a)
            } else if mint == pool_mint_b {
                (input_account_key, pool_state.token_account_b, &pool_acc_b)
            } else {
                return Err(format!("Provided token account is of incorrect mint").try_into()?);
            }
        }
    };

    let (out_source_key, out_source_acc) = if in_destination_key == pool_state.token_account_a {
        (pool_state.token_account_b, &pool_acc_b)
    } else {
        (pool_state.token_account_a, &pool_acc_a)
    };

    let out_destination_key = {
        // todo: possibility to set through CLI
        let mint = Pubkey::from_str(&out_source_acc.mint)?;
        get_associated_token_address(&payer_keypair.pubkey(), &mint)
    };

    let in_amount = matches.value_of("INPUT-AMOUNT")
        .map(|v| f64::from_str(v).map_err(|_| format!("Provided input amount is incorrect")))
        .ok_or(format!("Missing input amount"))?
        .map(|v| ui_amount_to_amount(v, in_destination_acc.token_amount.decimals))?;

    let expected_out_amount = hodor_program::swap::instruction::calculate_swap_amounts(
        u64::from_str(in_destination_acc.token_amount.amount.as_str())?,
        u64::from_str(out_source_acc.token_amount.amount.as_str())?,
        in_amount,
    ).ok_or(format!("Failed to calculate expected swap out amount"))?;

    // todo: slippage control through CLI, for now hardcoded 1%
    let mint_out_amount = expected_out_amount - (expected_out_amount / 100);

    println!("Expected received token amount: {}",
             amount_to_ui_amount(expected_out_amount, out_source_acc.token_amount.decimals));

    // todo: Prompt to accept expected amount

    let instruction = Instruction::new_with_bytes(
        context.program_id,
        &SwapInstruction::pack(&SwapInstruction::Swap {
            in_amount: in_amount,
            min_out_amount: mint_out_amount,
        }),
        vec![
            AccountMeta::new(payer_keypair.pubkey(), true),
            AccountMeta::new_readonly(pool_key, false),
            AccountMeta::new(in_source_key, false),
            AccountMeta::new(in_destination_key, false),
            AccountMeta::new(out_source_key, false),
            AccountMeta::new(out_destination_key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_keypair.pubkey()),
        &[&payer_keypair],
        context.rpc_client.get_latest_blockhash()?,
    );

    let transaction_result = context.rpc_client.send_and_confirm_transaction(&transaction);
    println!("Transaction {:?}", transaction_result);

    Ok(())
}

pub fn withdraw(context: Context, matches: &ArgMatches) -> Result<(), Error> {
    let pool_key = Pubkey::from_str(matches.value_of("POOL-ACCOUNT").unwrap())
        .map_err(|_| format!("Invalid swap pool account"))?;

    // todo: should be part of context
    let payer_keypair = read_keypair_file(&context.cli_config.keypair_path)?;

    let (pool_state, pool_account_a, pool_account_b)
        = get_pool_state_and_token_accounts(&context, &pool_key)?;

    let lp_account_key = get_associated_token_address(
        &payer_keypair.pubkey(),
        &pool_state.lp_mint);

    let lp_account = context.rpc_client.get_token_account_with_commitment(
        &lp_account_key, context.commitment)?
        .value.ok_or(format!("Unable to resolve source LP account: {}, mint: {}", lp_account_key, pool_state.lp_mint))?;

    let lp_amount = matches.value_of("LP-AMOUNT")
        .map_or_else(
            || u64::from_str(lp_account.token_amount.amount.as_str())
                .map_err(|_| format!("Unable to read available LP token amount")),
            |v| f64::from_str(v).map_err(|_| format!("Provided LP amount is incorrect"))
                .map(|v| ui_amount_to_amount(v, lp_account.token_amount.decimals)),
        )?;

    // todo: slippage

    let mint_a = Pubkey::from_str(&pool_account_a.mint)?;

    // todo: possibility to override through CLI param
    let destination_account_a_key = get_associated_token_address(&payer_keypair.pubkey(), &mint_a);

    let mint_b = Pubkey::from_str(&pool_account_b.mint)?;

    // todo: possibility to override through CLI param
    let destination_account_b_key = get_associated_token_address(&payer_keypair.pubkey(), &mint_b);

    // todo: option to create destination token accounts

    let instruction = Instruction::new_with_bytes(
        context.program_id,
        &SwapInstruction::pack(&SwapInstruction::Withdraw {
            lp_amount,
            min_a: 0,
            min_b: 0,
        }),
        vec![
            AccountMeta::new(payer_keypair.pubkey(), true),
            AccountMeta::new_readonly(pool_key, false),
            AccountMeta::new(pool_state.token_account_a, false),
            AccountMeta::new(destination_account_a_key, false),
            AccountMeta::new(pool_state.token_account_b, false),
            AccountMeta::new(destination_account_b_key, false),
            AccountMeta::new(pool_state.lp_mint, false),
            AccountMeta::new(lp_account_key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_keypair.pubkey()),
        &[&payer_keypair],
        context.rpc_client.get_latest_blockhash()?,
    );

    let transaction_result = context.rpc_client.send_and_confirm_transaction(&transaction);
    println!("Transaction {:?}", transaction_result);

    Ok(())
}

fn get_pool_state(context: &Context, pool_state_account: &Pubkey) -> Result<SwapPool, Error> {
    let account = context.rpc_client.get_account_with_commitment(
        pool_state_account,
        context.commitment,
    )?.value.ok_or(format!("Swap pool doesn't exist"))?;

    Ok(SwapPool::unpack(account.data())
        .map_err(|_| format!("Provided account is not a swap pool"))?)
}

fn get_pool_token_accounts(context: &Context, pool_state: &SwapPool) -> Result<(UiTokenAccount, UiTokenAccount), Error> {
    let pool_account_a = context.rpc_client.get_token_account_with_commitment(
        &pool_state.token_account_a, context.commitment)?
        .value.ok_or(format!("Failed to resolve pool token account A: {}", pool_state.token_account_a))?;

    let pool_account_b = context.rpc_client.get_token_account_with_commitment(
        &pool_state.token_account_b, context.commitment)?
        .value.ok_or(format!("Failed to resolve pool token account B: {}", pool_state.token_account_b))?;

    Ok((pool_account_a, pool_account_b))
}

fn get_pool_state_and_token_accounts(context: &Context, pool_key: &Pubkey)
                                     -> Result<(SwapPool, UiTokenAccount, UiTokenAccount), Error> {
    let state = get_pool_state(context, pool_key)?;
    let (acc_a, acc_b) = get_pool_token_accounts(context, &state)?;
    Ok((state, acc_a, acc_b))
}