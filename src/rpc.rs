use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Error};
use ckb_jsonrpc_types as json_types;
use ckb_sdk::rpc::ckb_light_client::{
    LightClientRpcClient, Order as JsonOrder, ScriptStatus, ScriptType, SearchKey, SearchKeyFilter,
};
use ckb_types::h256;
use clap::{Subcommand, ValueEnum};

use crate::common::{remove0x, HexH256};

#[derive(Subcommand, Debug)]
pub enum RpcCommands {
    SetScripts {
        /// The script status list
        ///
        /// The file data format (json):
        /// {
        ///   "script": {
        ///     "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
        ///     "hash_type": "type",
        ///     "args": "0x0000000000000000000000000000000000000000"
        ///   },
        ///   "script_type": "lock",
        ///   "block_number": "0xbb64"
        /// }
        #[arg(long, value_name = "FILE")]
        scripts: Vec<PathBuf>,

        /// Default will forbid empty script status list, use this flag to
        /// accept empty script status list.
        #[arg(long)]
        allow_empty: bool,
    },
    GetScripts,
    GetCells {
        /// The search key config, use `example-search-key` sub-command to generate a example value
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
        /// The search key config, use `example-search-key` sub-command to generate a example value
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
        /// The search key config, use `example-search-key` sub-command to generate a example value
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
        block_hash: HexH256,
    },
    GetTransaction {
        #[arg(long, value_name = "H256")]
        tx_hash: HexH256,
    },
    /// Fetch a header from remote node.
    ///
    /// Returns: FetchStatus<HeaderView>
    FetchHeader {
        #[arg(long, value_name = "H256")]
        block_hash: HexH256,
    },
    /// Fetch a transaction from remote node.
    ///
    /// Returns: FetchStatus<TransactionWithHeader>
    FetchTransaction {
        #[arg(long, value_name = "H256")]
        tx_hash: HexH256,
    },
    GetPeers,
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

pub fn invoke(rpc_url: &str, cmd: RpcCommands, debug: bool) -> Result<(), Error> {
    let mut client = LightClientRpcClient::new(rpc_url);
    match cmd {
        RpcCommands::SetScripts {
            scripts,
            allow_empty,
        } => {
            if scripts.is_empty() && !allow_empty {
                return Err(anyhow!(
                    "You may use `--allow-empty` flag to set empty script status list"
                ));
            }
            let scripts = scripts
                .into_iter()
                .map(|status| {
                    let content = fs::read_to_string(&status)?;
                    Ok(serde_json::from_str(&content)?)
                })
                .collect::<Result<Vec<ScriptStatus>, Error>>()?;
            if debug {
                println!(
                    "scripts: \n{}",
                    serde_json::to_string_pretty(&scripts).unwrap()
                );
            }
            client.set_scripts(scripts)?;
            println!("success!");
        }
        RpcCommands::GetScripts => {
            let scripts = client.get_scripts()?;
            println!("{}", serde_json::to_string_pretty(&scripts).unwrap());
        }
        RpcCommands::GetCells {
            search_key,
            order,
            limit,
            after,
        } => {
            let content = fs::read_to_string(&search_key)?;
            let search_key: SearchKey = serde_json::from_str(&content)?;
            let after = after
                .as_ref()
                .map(|s| remove0x(s))
                .map(|s| hex::decode(s).map(json_types::JsonBytes::from_vec))
                .transpose()
                .map_err(|err| anyhow!("parse `after` field error: {}", err))?;
            let page = client.get_cells(search_key, order.into(), limit.into(), after)?;
            println!("{}", serde_json::to_string_pretty(&page).unwrap());
        }
        RpcCommands::GetTransactions {
            search_key,
            order,
            limit,
            after,
        } => {
            let content = fs::read_to_string(&search_key)?;
            let search_key: SearchKey = serde_json::from_str(&content)?;
            let after = after
                .as_ref()
                .map(|s| remove0x(s))
                .map(|s| hex::decode(&s).map(json_types::JsonBytes::from_vec))
                .transpose()
                .map_err(|err| anyhow!("parse `after` field error: {}", err))?;
            let page = client.get_transactions(search_key, order.into(), limit.into(), after)?;
            println!("{}", serde_json::to_string_pretty(&page).unwrap());
        }
        RpcCommands::GetCellsCapacity { search_key } => {
            let content = fs::read_to_string(&search_key)?;
            let search_key: SearchKey = serde_json::from_str(&content)?;
            let cells_capacity = client.get_cells_capacity(search_key)?;
            println!("{}", serde_json::to_string_pretty(&cells_capacity).unwrap());
        }
        RpcCommands::SendTransaction { transaction } => {
            let content = fs::read_to_string(&transaction)?;
            let tx: json_types::Transaction = serde_json::from_str(&content)?;
            let tx_hash = client.send_transaction(tx)?;
            println!("Transaction sent!, hash: {:#x}", tx_hash);
        }
        RpcCommands::GetTipHeader => {
            let header = client.get_tip_header()?;
            println!("{}", serde_json::to_string_pretty(&header).unwrap());
        }
        RpcCommands::GetGenesisBlock => {
            let block = client.get_genesis_block()?;
            println!("{}", serde_json::to_string_pretty(&block).unwrap());
        }
        RpcCommands::GetHeader { block_hash } => {
            let value = client.get_header(block_hash.0)?;
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
        RpcCommands::GetTransaction { tx_hash } => {
            let value = client.get_transaction(tx_hash.0)?;
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
        RpcCommands::FetchHeader { block_hash } => {
            let value = client.fetch_header(block_hash.0)?;
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
        RpcCommands::FetchTransaction { tx_hash } => {
            let value = client.fetch_transaction(tx_hash.0)?;
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
        RpcCommands::GetPeers => {
            let peers = client.get_peers()?;
            println!("{}", serde_json::to_string_pretty(&peers).unwrap());
        }
    }
    Ok(())
}

pub fn print_example_search_key(with_filter: bool) {
    let mut search_key = SearchKey {
        script: json_types::Script {
            code_hash: h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8"),
            hash_type: json_types::ScriptHashType::Type,
            args: json_types::JsonBytes::from_vec(vec![0, 1, 2, 3]),
        },
        script_type: ScriptType::Lock,
        filter: None,
        with_data: Some(false),
        group_by_transaction: None,
    };
    if with_filter {
        search_key.filter = Some(SearchKeyFilter {
            script: Some(json_types::Script {
                code_hash: h256!(
                    "0x82d76d1b75fe2fd9a27dfbaa65a039221a380d76c926f378d3f81cf3e7e13f2e"
                ),
                hash_type: json_types::ScriptHashType::Type,
                args: json_types::JsonBytes::from_vec(vec![0, 1, 2, 3]),
            }),
            script_len_range: None,
            output_data_len_range: Some([22.into(), 888.into()]),
            output_capacity_range: Some([1000000.into(), 100000000.into()]),
            block_range: Some([33.into(), 999.into()]),
        });
    }
    println!("{}", serde_json::to_string_pretty(&search_key).unwrap());
}
