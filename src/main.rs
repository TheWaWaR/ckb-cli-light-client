use std::error::Error as StdErr;
use std::path::PathBuf;

use ckb_sdk::types::{Address, HumanCapacity};
use ckb_types::H256;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Transfer some capacity from given address to a receiver address
    Transfer {
        #[arg(long, value_name = "ADDR")]
        from_address: Address,
        #[arg(long, value_name = "ADDR")]
        to_address: Address,
        #[arg(long, value_name = "CAPACITY")]
        capacity: HumanCapacity,
        #[arg(long)]
        skip_check_to_address: bool,
    },

    /// Nervos DAO operations
    #[command(subcommand)]
    Dao(DaoCommands),

    /// Send jsonrpc call the ckb-light-client rpc server
    #[command(subcommand)]
    Rpc(RpcCommands),
}

#[derive(ValueEnum, Eq, PartialEq, Clone, Copy, Debug)]
pub enum Order {
    Desc,
    Asc,
}

#[derive(Subcommand, Debug)]
enum RpcCommands {
    SetScripts {
        #[arg(long)]
        scripts: Vec<PathBuf>,
    },
    GetScripts,
    GetCells {
        #[arg(long, value_name = "FILE")]
        search_key: PathBuf,
        #[arg(long, value_enum)]
        order: Order,
        #[arg(long, value_name = "NUM")]
        limit: u32,
    },
    GetTransactions {
        #[arg(long, value_name = "FILE")]
        search_key: PathBuf,
        #[arg(long, value_enum)]
        order: Order,
        #[arg(long, value_name = "NUM")]
        limit: u32,
    },
    GetCellsCapacity {
        #[arg(long, value_name = "FILE")]
        search_key: PathBuf,
    },
    SendTransaction {
        #[arg(long, value_name = "FILE")]
        transaction: PathBuf,
    },
    GetTipHeader,
    GetHeader {
        #[arg(long, value_name = "H256")]
        block_hash: H256,
    },
    GetTransaction {
        #[arg(long, value_name = "H256")]
        tx_hash: H256,
    },
    /// Fetch a header from remote node.
    ///
    /// Returns: FetchStatus<HeaderView>
    FetchHeader {
        #[arg(long, value_name = "H256")]
        block_hash: H256,
    },
    /// Fetch a transaction from remote node.
    ///
    /// Returns: FetchStatus<TransactionWithHeader>
    FetchTransaction {
        #[arg(long, value_name = "H256")]
        tx_hash: H256,
    },
    /// Remove fetched headers. (if `block_hashes` is None remove all headers)
    ///
    /// Returns:
    ///   * The removed block hashes
    RemoveHeaders {
        #[arg(long)]
        block_hashes: Option<Vec<H256>>,
    },
    /// Remove fetched transactions. (if `tx_hashes` is None remove all transactions)
    ///
    /// Returns:
    ///   * The removed transaction hashes
    RemoveTransactions {
        #[arg(long)]
        tx_hashes: Option<Vec<H256>>,
    },
    GetPeers,
}

#[derive(Subcommand, Debug)]
enum DaoCommands {
    /// Deposit capacity into NervosDAO
    Deposit {
        #[arg(long, value_name = "ADDR")]
        from_address: Address,
        #[arg(long, value_name = "CAPACITY")]
        capacity: HumanCapacity,
    },
    /// Prepare specified cells from NervosDAO
    Prepare {
        #[arg(long, value_name = "ADDR")]
        from_address: Address,

        #[arg(long, value_name = "CAPACITY")]
        /// out-point to specify a cell. Example: 0xd56ed5d4e8984701714de9744a533413f79604b3b91461e2265614829d2005d1-1
        out_point: Vec<String>,
    },
    /// Query NervosDAO deposited capacity by address
    QueryDepositedCells {
        #[arg(long, value_name = "ADDR")]
        address: Address,
    },
    /// Query NervosDAO prepared capacity by address
    QueryPreparedCells {
        #[arg(long, value_name = "ADDR")]
        address: Address,
    },
}

fn main() -> Result<(), Box<dyn StdErr>> {
    let cli = Cli::parse();
    println!("cli: {:#?}", cli);
    Ok(())
}
