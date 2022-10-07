use anyhow::{anyhow, Error};
use ckb_sdk::{
    rpc::ckb_light_client::{LightClientRpcClient, ScriptStatus},
    types::Address,
};
use ckb_types::packed::Script;
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
        _ => {
            println!("rpc cmd: {:#?}", cmd);
            return Err(anyhow!("not yet implemented"));
        }
    }
    Ok(())
}
