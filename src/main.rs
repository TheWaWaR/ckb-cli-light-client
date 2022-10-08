use std::error::Error as StdErr;
use std::path::PathBuf;

use anyhow::anyhow;
use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    rpc::ckb_light_client::{LightClientRpcClient, Order as JsonOrder},
    types::{Address, HumanCapacity},
};
use ckb_types::H256;
use clap::{ArgGroup, Parser, Subcommand, ValueEnum};

mod rpc;
mod wallet;

use wallet::{build_transfer_tx, get_capacity};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Cli {
    /// CKB light client rpc url
    #[clap(long, value_name = "URL", default_value = "http://127.0.0.1:9000")]
    rpc: String,

    #[clap(long)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Get capacity of an address
    GetCapacity {
        #[arg(long, value_name = "ADDR")]
        address: Address,
    },
    /// Transfer some capacity from given address to a receiver address
    #[command(group(ArgGroup::new("from").required(true).args(["from_address", "from_key"])))]
    Transfer {
        /// The sender address (sighash only, also used to match key in ckb-cli keystore)
        #[arg(long, value_name = "ADDR")]
        from_address: Option<Address>,

        /// The sender private key (hex string, also used to generate sighash address)
        #[arg(long, value_name = "PRIVKEY")]
        from_key: Option<H256>,

        /// The receiver address
        #[arg(long, value_name = "ADDR")]
        to_address: Address,

        /// The capacity to transfer (unit: CKB, example: 102.43)
        #[arg(long, value_name = "CAPACITY")]
        capacity: HumanCapacity,

        #[arg(long)]
        skip_check_to_address: bool,
    },

    /// Nervos DAO operations
    #[command(subcommand)]
    Dao(DaoCommands),

    /// Output the example `SearchKey` value
    ExampleSearchKey {
        /// With example `SearchKeyFilter` value
        #[arg(long)]
        with_filter: bool,
    },

    /// Send jsonrpc call the ckb-light-client rpc server
    #[command(subcommand)]
    Rpc(RpcCommands),
}

#[derive(ValueEnum, Eq, PartialEq, Clone, Copy, Debug)]
pub enum Order {
    Desc,
    Asc,
}

impl From<Order> for JsonOrder {
    fn from(value: Order) -> JsonOrder {
        match value {
            Order::Asc => JsonOrder::Asc,
            Order::Desc => JsonOrder::Desc,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum RpcCommands {
    SetScripts {
        /// The script status list (format: "ADDR,NUM", example: "ckt1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq,5896000")
        #[arg(long, value_name = "SCRIPT_STATUS")]
        scripts: Vec<String>,
    },
    GetScripts,
    GetCells {
        #[arg(long, value_name = "FILE")]
        search_key: PathBuf,
        #[arg(long, value_enum, default_value = "asc")]
        order: Order,
        #[arg(long, value_name = "NUM", default_value = "20")]
        limit: u32,
        #[arg(long, value_name = "HEX")]
        after: Option<String>,
    },
    GetTransactions {
        #[arg(long, value_name = "FILE")]
        search_key: PathBuf,
        #[arg(long, value_enum, default_value = "asc")]
        order: Order,
        #[arg(long, value_name = "NUM", default_value = "20")]
        limit: u32,
        #[arg(long, value_name = "HEX")]
        after: Option<String>,
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
    GetGenesisBlock,
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
    match cli.command {
        Commands::GetCapacity { address } => {
            let capacity = get_capacity(cli.rpc.as_str(), address)?;
            println!("capacity: {} CKB", HumanCapacity(capacity));
        }
        Commands::Transfer {
            from_address,
            from_key,
            to_address,
            capacity,
            skip_check_to_address,
        } => {
            let from_key = from_key
                .map(|data| {
                    secp256k1::SecretKey::from_slice(data.as_bytes())
                        .map_err(|err| anyhow!("invalid from key: {}", err))
                })
                .transpose()?;
            let tx = build_transfer_tx(
                cli.rpc.as_str(),
                from_address,
                from_key,
                to_address,
                capacity.0,
                skip_check_to_address,
            )?;
            // Send transaction
            let json_tx = json_types::TransactionView::from(tx);
            if cli.debug {
                println!("tx: {}", serde_json::to_string_pretty(&json_tx).unwrap());
            }
            let tx_hash = LightClientRpcClient::new(cli.rpc.as_str())
                .send_transaction(json_tx.inner)
                .expect("send transaction");
            println!(">>> tx sent! {:#x} <<<", tx_hash);
        }
        Commands::Dao(dao) => {
            println!("dao: {:#?}", dao);
            return Err(anyhow!("not yet implemented").into());
        }
        Commands::ExampleSearchKey { with_filter } => {
            rpc::print_example_search_key(with_filter);
        }
        Commands::Rpc(cmd) => {
            rpc::invoke(cli.rpc.as_str(), cmd)?;
        }
    }
    Ok(())
}
