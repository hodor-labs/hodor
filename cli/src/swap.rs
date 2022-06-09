use std::str::FromStr;
use clap::ArgMatches;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_sdk::account::ReadableAccount;
use solana_sdk::signature::{Keypair, read_keypair_file};
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
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



    todo!()
}

pub fn print_info(context: Context, matches: &ArgMatches) -> Result<(), Error> {
    let pool_key = Pubkey::from_str(matches.value_of("POOL-ACCOUNT").unwrap())
        .map_err(|_| format!("Invalid swap pool account"))?;

    let pool_account = context.rpc_client.get_account_with_commitment(
        &pool_key,
        context.commitment,
    )?.value.ok_or(format!("Swap pool doesn't exist"))?;

    let pool_state = SwapPool::unpack(pool_account.data())
        .map_err(|_| format!("Provided account is not a swap pool"))?;

    let token_acc_a = context.rpc_client.get_token_account_with_commitment(
        &pool_state.token_account_a,
        context.commitment,
    )?.value.ok_or(format!("Unexpected, unable to resolve token account: {}", pool_state.token_account_a))?;

    let token_acc_b = context.rpc_client.get_token_account_with_commitment(
        &pool_state.token_account_b,
        context.commitment,
    )?.value.ok_or(format!("Unexpected, unable to resolve token account: {}", pool_state.token_account_b))?;

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