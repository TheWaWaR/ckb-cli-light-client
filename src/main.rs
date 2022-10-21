use std::error::Error as StdErr;

use ckb_sdk::types::{Address, HumanCapacity};
use clap::{ArgGroup, Parser, Subcommand};

mod common;
mod dao;
mod rpc;
mod wallet;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Cli {
    /// CKB light client rpc url
    #[clap(long, value_name = "URL", default_value = "http://127.0.0.1:9000")]
    rpc: String,

    /// Debug mode, print more information
    #[clap(long)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Get capacity of an address
    GetCapacity {
        /// The address
        #[arg(long, value_name = "ADDR")]
        address: Address,
    },
    /// Transfer some capacity from given address to a receiver address
    #[command(group(ArgGroup::new("from").required(true).args(["from_address", "from_key"])))]
    Transfer {
        /// The sender address (sighash only, also be used to match key in ckb-cli keystore)
        #[arg(long, value_name = "ADDR")]
        from_address: Option<Address>,

        /// The sender private key (hex string, also be used to generate sighash address)
        #[arg(long, value_name = "PRIVKEY")]
        from_key: Option<common::HexH256>,

        /// The receiver address
        #[arg(long, value_name = "ADDR")]
        to_address: Address,

        /// The capacity to transfer (unit: CKB, example: 102.43)
        #[arg(long, value_name = "CAPACITY")]
        capacity: HumanCapacity,

        /// Skip check <to-address> (default only allow sighash/multisig address), be cautious to use this flag
        #[arg(long)]
        skip_check_to_address: bool,
    },

    /// Nervos DAO operations
    #[command(subcommand)]
    Dao(dao::DaoCommands),

    /// Output the example `SearchKey` value
    #[command(group(ArgGroup::new("rpc-method").required(false).args(["get_transactions", "get_cells", "get_cells_capacity"])))]
    ExampleSearchKey {
        /// With example `SearchKeyFilter` value
        #[arg(long)]
        with_filter: bool,
        /// Set appropriate default `SearchKeyFilter` for `get_transactions` RPC method
        #[arg(long)]
        get_transactions: bool,
        /// Set appropriate default `SearchKeyFilter` for `get_cells` RPC method
        #[arg(long)]
        get_cells: bool,
        /// Set appropriate default `SearchKeyFilter` for `get_cells_capacity` RPC method
        #[arg(long)]
        get_cells_capacity: bool,
    },

    /// Send jsonrpc call the ckb-light-client rpc server
    #[command(subcommand)]
    Rpc(rpc::RpcCommands),
}

fn main() -> Result<(), Box<dyn StdErr>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::GetCapacity { address } => {
            wallet::get_capacity(cli.rpc.as_str(), address)?;
        }
        Commands::Transfer {
            from_address,
            from_key,
            to_address,
            capacity,
            skip_check_to_address,
        } => {
            wallet::transfer(
                cli.rpc.as_str(),
                from_address,
                from_key.map(|v| v.0),
                to_address,
                capacity.0,
                skip_check_to_address,
                cli.debug,
            )?;
        }
        Commands::Dao(cmd) => {
            dao::invoke(cli.rpc.as_str(), cmd, cli.debug)?;
        }
        Commands::ExampleSearchKey {
            with_filter,
            get_transactions,
            get_cells,
            get_cells_capacity,
        } => {
            rpc::print_example_search_key(
                with_filter,
                get_transactions,
                get_cells,
                get_cells_capacity,
            );
        }
        Commands::Rpc(cmd) => {
            rpc::invoke(cli.rpc.as_str(), cmd, cli.debug)?;
        }
    }
    Ok(())
}
