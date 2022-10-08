use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{anyhow, Error};
use byteorder::{ByteOrder, LittleEndian};
use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    constants::{DAO_TYPE_HASH, SIGHASH_TYPE_HASH},
    rpc::LightClientRpcClient,
    traits::{
        CellCollector, CellQueryOptions, DefaultCellDepResolver, LightClientCellCollector,
        LightClientHeaderDepResolver, LightClientTransactionDependencyProvider, LiveCell, Signer,
        ValueRangeOption,
    },
    tx_builder::{
        dao::{
            DaoDepositBuilder, DaoDepositReceiver, DaoPrepareBuilder, DaoPrepareItem,
            DaoWithdrawBuilder, DaoWithdrawItem, DaoWithdrawReceiver,
        },
        CapacityBalancer, CapacityProvider, TxBuilder,
    },
    unlock::{ScriptUnlocker, SecpSighashScriptSigner, SecpSighashUnlocker},
    Address, HumanCapacity, ScriptId,
};
use ckb_types::{
    bytes::Bytes,
    core::{FeeRate, ScriptHashType},
    packed::{CellInput, OutPoint, Script, WitnessArgs},
    prelude::*,
    H256,
};
use clap::{ArgGroup, Subcommand};
use serde::Serialize;

use super::wallet::get_signer;

#[derive(Subcommand, Debug)]
pub enum DaoCommands {
    /// Deposit capacity into NervosDAO
    #[command(group(ArgGroup::new("from").required(true).args(["from_address", "from_key"])))]
    Deposit {
        /// The sender address (sighash only, also used to match key in ckb-cli keystore)
        #[arg(long, value_name = "ADDR")]
        from_address: Option<Address>,

        /// The sender private key (hex string, also used to generate sighash address)
        #[arg(long, value_name = "PRIVKEY")]
        from_key: Option<H256>,

        /// The capacity to deposit (unit: CKB, example: 102.43)
        #[arg(long, value_name = "CAPACITY")]
        capacity: HumanCapacity,
    },
    /// Prepare specified cells from NervosDAO
    #[command(group(ArgGroup::new("from").required(true).args(["from_address", "from_key"])))]
    Prepare {
        /// The sender address (sighash only, also used to match key in ckb-cli keystore)
        #[arg(long, value_name = "ADDR")]
        from_address: Option<Address>,

        /// The sender private key (hex string, also used to generate sighash address)
        #[arg(long, value_name = "PRIVKEY")]
        from_key: Option<H256>,

        #[arg(long, value_name = "OUT-POINT")]
        /// out-point to specify a cell. Example: 0xd56ed5d4e8984701714de9744a533413f79604b3b91461e2265614829d2005d1-1
        out_points: Vec<String>,
    },
    /// Withdraw specified cells from NervosDAO
    #[command(group(ArgGroup::new("from").required(true).args(["from_address", "from_key"])))]
    Withdraw {
        /// The sender address (sighash only, also used to match key in ckb-cli keystore)
        #[arg(long, value_name = "ADDR")]
        from_address: Option<Address>,

        /// The sender private key (hex string, also used to generate sighash address)
        #[arg(long, value_name = "PRIVKEY")]
        from_key: Option<H256>,

        #[arg(long, value_name = "OUT-POINT")]
        /// out-point to specify a cell. Example: 0xd56ed5d4e8984701714de9744a533413f79604b3b91461e2265614829d2005d1-1
        out_points: Vec<String>,
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

pub fn invoke(rpc_url: &str, cmd: DaoCommands, debug: bool) -> Result<(), Error> {
    match cmd {
        DaoCommands::Deposit {
            from_address,
            from_key,
            capacity,
        } => {
            let (sender, signer) = get_signer(from_address, from_key)?;
            let deposit_receiver = DaoDepositReceiver::new(sender.clone(), capacity.0);
            let tx_builder = DaoDepositBuilder::new(vec![deposit_receiver]);
            build_and_send_dao_tx(&tx_builder, sender, signer, rpc_url, debug)?;
        }
        DaoCommands::Prepare {
            from_address,
            from_key,
            out_points,
        } => {
            let (sender, signer) = get_signer(from_address, from_key)?;
            let items = parse_out_points(out_points)?
                .into_iter()
                .map(|out_point| DaoPrepareItem::from(CellInput::new(out_point, 0)))
                .collect();
            let tx_builder = DaoPrepareBuilder::new(items);
            build_and_send_dao_tx(&tx_builder, sender, signer, rpc_url, debug)?;
        }
        DaoCommands::Withdraw {
            from_address,
            from_key,
            out_points,
        } => {
            let (sender, signer) = get_signer(from_address, from_key)?;
            let mut items: Vec<_> = parse_out_points(out_points)?
                .into_iter()
                .map(|out_point| DaoWithdrawItem::new(out_point, None))
                .collect();
            items[0].init_witness = Some(
                WitnessArgs::new_builder()
                    .lock(Some(Bytes::from(vec![0u8; 65])).pack())
                    .build(),
            );
            let receiver = DaoWithdrawReceiver::LockScript {
                script: sender.clone(),
                fee_rate: Some(FeeRate::from_u64(1000)),
            };
            let tx_builder = DaoWithdrawBuilder::new(items, receiver);
            build_and_send_dao_tx(&tx_builder, sender, signer, rpc_url, debug)?;
        }
        DaoCommands::QueryDepositedCells { address } => {
            let cells = query_dao_cells(rpc_url, &address, true)?;
            let total_capacity = cells.iter().map(|info| info.capacity).sum::<u64>();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "live_cells": cells,
                    "total_capacity": total_capacity,
                }),)
                .unwrap()
            );
        }
        DaoCommands::QueryPreparedCells { address } => {
            let cells = query_dao_cells(rpc_url, &address, false)?;
            let total_capacity = cells.iter().map(|info| info.capacity).sum::<u64>();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "live_cells": cells,
                    "total_capacity": total_capacity,
                }),)
                .unwrap()
            );
        }
    }
    Ok(())
}

fn build_and_send_dao_tx(
    builder: &dyn TxBuilder,
    sender: Script,
    signer: Box<dyn Signer>,
    rpc_url: &str,
    debug: bool,
) -> Result<(), Error> {
    let balancer = CapacityBalancer {
        fee_rate: FeeRate::from_u64(1000),
        change_lock_script: None,
        capacity_provider: CapacityProvider::new_simple(vec![(
            sender,
            WitnessArgs::new_builder()
                .lock(Some(Bytes::from(vec![0u8; 65])).pack())
                .build(),
        )]),
        force_small_change_as_fee: None,
    };

    let script_id = ScriptId::new_type(SIGHASH_TYPE_HASH.clone());
    let sighash_unlocker = SecpSighashUnlocker::new(SecpSighashScriptSigner::new(signer));
    let mut unlockers: HashMap<_, Box<dyn ScriptUnlocker>> = HashMap::new();
    unlockers.insert(script_id, Box::new(sighash_unlocker));

    // Build:
    //   * CellDepResolver
    //   * HeaderDepResolver
    //   * CellCollector
    //   * TransactionDependencyProvider
    let mut client = LightClientRpcClient::new(rpc_url);
    let genesis_block = client.get_genesis_block()?.into();
    let cell_dep_resolver = DefaultCellDepResolver::from_genesis(&genesis_block)?;
    let header_dep_resolver = LightClientHeaderDepResolver::new(rpc_url);
    let tx_dep_provider = LightClientTransactionDependencyProvider::new(rpc_url);
    let mut cell_collector = LightClientCellCollector::new(rpc_url);

    let (tx, still_locked_groups) = builder.build_unlocked(
        &mut cell_collector,
        &cell_dep_resolver,
        &header_dep_resolver,
        &tx_dep_provider,
        &balancer,
        &unlockers,
    )?;
    assert!(still_locked_groups.is_empty());

    // Send transaction
    let json_tx = json_types::TransactionView::from(tx);
    if debug {
        println!("tx: {}", serde_json::to_string_pretty(&json_tx).unwrap());
    }
    let tx_hash = LightClientRpcClient::new(rpc_url)
        .send_transaction(json_tx.inner)
        .expect("send transaction");
    println!(">>> tx sent! {:#x} <<<", tx_hash);
    Ok(())
}

fn parse_out_points(out_points: Vec<String>) -> Result<Vec<OutPoint>, Error> {
    if out_points.is_empty() {
        return Err(anyhow!("missing out poinst"));
    }
    out_points
        .into_iter()
        .map(|input| {
            let parts = input.split('-').collect::<Vec<_>>();
            if parts.len() != 2 {
                return Err(anyhow!(
                    "Invalid OutPoint: {}, format: {{tx-hash}}-{{index}}",
                    input
                ));
            }
            let tx_hash_str = if let Some(stripped) = parts[0].strip_prefix("0x") {
                stripped
            } else {
                parts[0]
            };
            let tx_hash: H256 = H256::from_str(tx_hash_str)?;
            let index = u32::from_str(parts[1])?;
            Ok(OutPoint::new(tx_hash.pack(), index))
        })
        .collect::<Result<Vec<_>, Error>>()
}

// LiveCell index in a block
#[derive(Serialize)]
pub struct CellIndex {
    pub tx_index: u32,
    pub output_index: u32,
}
#[derive(Serialize)]
struct LiveCellInfo {
    pub tx_hash: H256,
    pub output_index: u32,
    pub data_bytes: u64,
    pub lock_hash: H256,
    // Type script's code_hash and script_hash
    pub type_hashes: Option<(H256, H256)>,
    // Capacity
    pub capacity: u64,
    // Block number
    pub number: u64,
    // Location in the block
    pub index: CellIndex,
}
fn to_live_cell_info(cell: &LiveCell) -> LiveCellInfo {
    let output_index: u32 = cell.out_point.index().unpack();
    LiveCellInfo {
        tx_hash: cell.out_point.tx_hash().unpack(),
        output_index,
        data_bytes: cell.output_data.len() as u64,
        lock_hash: cell.output.lock().calc_script_hash().unpack(),
        type_hashes: cell.output.type_().to_opt().map(|type_script| {
            (
                type_script.code_hash().unpack(),
                type_script.calc_script_hash().unpack(),
            )
        }),
        capacity: cell.output.capacity().unpack(),
        number: cell.block_number,
        index: CellIndex {
            tx_index: cell.tx_index,
            output_index,
        },
    }
}

fn query_dao_cells(
    rpc_url: &str,
    address: &Address,
    is_deposit: bool,
) -> Result<Vec<LiveCellInfo>, Error> {
    let dao_type_script = Script::new_builder()
        .code_hash(DAO_TYPE_HASH.pack())
        .hash_type(ScriptHashType::Type.into())
        .build();
    let mut query = CellQueryOptions::new_lock(Script::from(address));
    query.secondary_script = Some(dao_type_script);
    query.data_len_range = Some(ValueRangeOption::new_exact(8));
    query.min_total_capacity = u64::max_value();

    let mut cell_collector = LightClientCellCollector::new(rpc_url);
    let (cells, _) = cell_collector.collect_live_cells(&query, false)?;
    let cell_filter = if is_deposit {
        |block_number| block_number == 0
    } else {
        |block_number| block_number != 0
    };
    Ok(cells
        .iter()
        .filter(|cell| cell_filter(LittleEndian::read_u64(&cell.output_data.as_ref()[0..8])))
        .map(to_live_cell_info)
        .collect::<Vec<_>>())
}
