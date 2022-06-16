mod swap;

use clap::{Arg, Command};
use solana_cli_config::Config;
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;

pub(crate) type Error = Box<dyn std::error::Error>;

pub struct Context {
    pub cli_config: Config,
    pub rpc_client: RpcClient,
    pub commitment: CommitmentConfig,
    pub program_id: Pubkey,
}

fn main() {
    let cmd = Command::new("hodor")
        .bin_name("hodor")
        .subcommand_required(true)
        .subcommand(
            Command::new("swap")
                .subcommand_required(true)
                .subcommand(
                    Command::new("create")
                        .about("Create new swap pool")
                        .arg(Arg::new("TOKEN-A").required(true).index(1))
                        .arg(Arg::new("TOKEN-B").required(true).index(2))
                    // todo: fee - defaults to 0
                )
                .subcommand(
                    Command::new("deposit")
                        .about("Depositing tokens to swap pool")
                        .arg(Arg::new("POOL-ACCOUNT").required(true).index(1))
                        .arg(Arg::new("AMOUNT-A").required(true).index(2))
                        .arg(Arg::new("AMOUNT-B").required(true).index(3))
                )
                .subcommand(
                    Command::new("info")
                        .about("Get details of swap pool")
                        .arg(Arg::new("POOL-ACCOUNT").required(true).index(1))
                )
                .subcommand(
                    Command::new("swap")
                        .about("Swap tokens")
                        .arg(Arg::new("POOL-ACCOUNT").required(true).index(1))
                        .arg(Arg::new("INPUT-ACCOUNT").required(true).index(2)
                            .long_help("Mint or mint token account address which will be used as swap input. \
                            If mint is provided associated token account will be used.")
                        )
                        .arg(
                            Arg::new("INPUT-AMOUNT").required(true).index(3)
                                .long_help("Amount of tokens to swap")
                        )
                )
                .subcommand(
                    Command::new("withdraw")
                        .about("Withdraw tokens from swap pool")
                        .arg(Arg::new("POOL-ACCOUNT").required(true).index(1))
                        .arg(Arg::new("LP-AMOUNT").required(false).index(2))
                )
        );
    let matches = cmd.get_matches();

    // todo: argument to override config file path
    // todo: try first '~/.config/hodor/cli/config.yml'
    let config_file = solana_cli_config::CONFIG_FILE.as_ref().unwrap();
    let cli_config = Config::load(config_file).unwrap();

    // todo: respect commitment level out of config
    let rpc_client = RpcClient::new_with_commitment(
        &cli_config.json_rpc_url, CommitmentConfig::processed());

    // todo: parametrized through arg
    let program_id = hodor_program::id();

    let context = Context {
        cli_config,
        rpc_client,
        commitment: CommitmentConfig::processed(), // todo: parametrize it or/and read from CLI config
        program_id,
    };

    let result = match matches.subcommand() {
        Some(("swap", matches)) => {
            match matches.subcommand() {
                Some(("create", matches)) => {
                    swap::create_pool(context, matches)
                }
                Some(("deposit", matches)) => {
                    swap::deposit(context, matches)
                }
                Some(("info", matches)) => {
                    swap::print_info(context, matches)
                }
                Some(("swap", matches)) => {
                    swap::swap(context, matches)
                }
                Some(("withdraw", matches)) => {
                    swap::withdraw(context, matches)
                }
                _ => unreachable!()
            }
        }
        _ => unreachable!(),
    };

    if let Err(error) = result {
        println!("{}", error);
    }
}