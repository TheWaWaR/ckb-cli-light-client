use anyhow::{anyhow, Error};
use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    rpc::ckb_light_client::{
        LightClientRpcClient, ScriptStatus, ScriptType, SearchKey, SearchKeyFilter,
    },
    types::Address,
};
use ckb_types::{h256, packed::Script};
use std::fs;
use std::str::FromStr;

use super::RpcCommands;

pub fn invoke(rpc_url: &str, cmd: RpcCommands) -> Result<(), Error> {
    let mut client = LightClientRpcClient::new(rpc_url);
    match cmd {
        RpcCommands::SetScripts { scripts } => {
            let scripts = scripts
                .into_iter()
                .map(|status| {
                    let parts = status.split(',').collect::<Vec<_>>();
                    if parts.len() != 2 {
                        return Err(anyhow!("invalid script status: {}", status));
                    }
                    let address = Address::from_str(parts[0])
                        .map_err(|err| anyhow!("parse script status address error: {}", err))?;
                    let script: ckb_jsonrpc_types::Script = Script::from(&address).into();
                    let block_number = u64::from_str(parts[1]).map_err(|err| {
                        anyhow!("parse script status block number error: {}", err)
                    })?;
                    Ok(ScriptStatus {
                        script,
                        block_number: block_number.into(),
                    })
                })
                .collect::<Result<Vec<ScriptStatus>, Error>>()?;
            println!(
                "scripts: \n{}",
                serde_json::to_string_pretty(&scripts).unwrap()
            );
            client.set_scripts(scripts)?;
            println!("success!");
        }
        RpcCommands::GetScripts => {
            let scripts = client.get_scripts()?;
            println!(
                "scripts: \n{}",
                serde_json::to_string_pretty(&scripts).unwrap()
            );
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
                .map(|s| {
                    if let Some(stripped) = s.strip_prefix("0x") {
                        stripped
                    } else {
                        &s[..]
                    }
                })
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
                .map(|s| {
                    if let Some(stripped) = s.strip_prefix("0x") {
                        stripped
                    } else {
                        &s[..]
                    }
                })
                .map(|s| hex::decode(&s).map(json_types::JsonBytes::from_vec))
                .transpose()
                .map_err(|err| anyhow!("parse `after` field error: {}", err))?;
            let page = client.get_transactions(search_key, order.into(), limit.into(), after)?;
            println!("{}", serde_json::to_string_pretty(&page).unwrap());
        }
        RpcCommands::GetCellsCapacity { search_key } => {
            let content = fs::read_to_string(&search_key)?;
            let search_key: SearchKey = serde_json::from_str(&content)?;
            let capacity: u64 = client.get_cells_capacity(search_key)?.value();
            println!("capacity: {}", capacity);
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
            let value = client.get_header(block_hash)?;
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
        RpcCommands::GetTransaction { tx_hash } => {
            let value = client.get_transaction(tx_hash)?;
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
        RpcCommands::FetchHeader { block_hash } => {
            let value = client.fetch_header(block_hash)?;
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
        RpcCommands::FetchTransaction { tx_hash } => {
            let value = client.fetch_transaction(tx_hash)?;
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
        RpcCommands::RemoveHeaders { block_hashes } => {
            let value = client.remove_headers(block_hashes)?;
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
        RpcCommands::RemoveTransactions { tx_hashes } => {
            let value = client.remove_transactions(tx_hashes)?;
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
            output_data_len_range: Some([22.into(), 888.into()]),
            output_capacity_range: Some([1000000.into(), 100000000.into()]),
            block_range: Some([33.into(), 999.into()]),
        });
    }
    println!("{}", serde_json::to_string_pretty(&search_key).unwrap());
}
