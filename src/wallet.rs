use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use anyhow::{anyhow, Error};
use ckb_hash::blake2b_256;
use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    constants::{MULTISIG_TYPE_HASH, SIGHASH_TYPE_HASH},
    rpc::{
        ckb_light_client::{ScriptType, SearchKey},
        LightClientRpcClient,
    },
    traits::{
        DefaultCellDepResolver, LightClientCellCollector, LightClientHeaderDepResolver,
        LightClientTransactionDependencyProvider, SecpCkbRawKeySigner, Signer,
    },
    tx_builder::{transfer::CapacityTransferBuilder, CapacityBalancer, TxBuilder},
    unlock::{ScriptUnlocker, SecpSighashUnlocker},
    Address, HumanCapacity, ScriptId, SECP256K1,
};
use ckb_signer::{FileSystemKeystoreSigner, KeyStore, ScryptType};
use rpassword::prompt_password;

use ckb_types::{
    bytes::Bytes,
    core::{ScriptHashType, TransactionView},
    packed::{CellOutput, Script, WitnessArgs},
    prelude::*,
    H160, H256,
};

pub fn get_capacity(rpc_url: &str, address: Address) -> Result<(), Error> {
    let mut client = LightClientRpcClient::new(rpc_url);
    let script = Script::from(&address).into();
    if !client
        .get_scripts()?
        .iter()
        .any(|status| status.script == script)
    {
        return Err(anyhow!("address not registered, you may use `rpc set-scripts` subcommand to register the address"));
    }
    let search_key = SearchKey {
        script,
        script_type: ScriptType::Lock,
        filter: None,
        group_by_transaction: None,
    };
    let capacity: u64 = client.get_cells_capacity(search_key)?.value();
    println!("capacity: {} CKB", HumanCapacity(capacity));
    Ok(())
}

pub fn transfer(
    rpc_url: &str,
    from_address: Option<Address>,
    from_key: Option<H256>,
    to_address: Address,
    capacity: u64,
    skip_check_to_address: bool,
    debug: bool,
) -> Result<(), Error> {
    let tx = build_transfer_tx(
        rpc_url,
        from_address,
        from_key,
        to_address,
        capacity,
        skip_check_to_address,
    )?;
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

pub fn build_transfer_tx(
    rpc_url: &str,
    from_address: Option<Address>,
    from_key: Option<H256>,
    to_address: Address,
    capacity: u64,
    skip_check_to_address: bool,
) -> Result<TransactionView, Error> {
    let (sender, signer) = get_signer(from_address, from_key)?;
    let sighash_unlocker = SecpSighashUnlocker::from(signer);
    let sighash_script_id = ScriptId::new_type(SIGHASH_TYPE_HASH.clone());
    let mut unlockers = HashMap::default();
    unlockers.insert(
        sighash_script_id,
        Box::new(sighash_unlocker) as Box<dyn ScriptUnlocker>,
    );

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

    // Build CapacityBalancer
    let placeholder_witness = WitnessArgs::new_builder()
        .lock(Some(Bytes::from(vec![0u8; 65])).pack())
        .build();
    let balancer = CapacityBalancer::new_simple(sender, placeholder_witness, 1000);

    // Build the transaction
    let receiver = Script::from(&to_address);
    let to_address_hash_type = to_address.payload().hash_type();
    let to_address_code_hash: H256 = to_address
        .payload()
        .code_hash(Some(to_address.network()))
        .unpack();
    let to_address_args_len = to_address.payload().args().len();
    if !(skip_check_to_address
        || (to_address_hash_type == ScriptHashType::Type
            && to_address_code_hash == SIGHASH_TYPE_HASH
            && to_address_args_len == 20)
        || (to_address_hash_type == ScriptHashType::Type
            && to_address_code_hash == MULTISIG_TYPE_HASH
            && (to_address_args_len == 20 || to_address_args_len == 28)))
    {
        return Err(anyhow!("Invalid to-address: {}\n[Hint]: Add `--skip-check-to-address` flag to transfer to any address", to_address));
    }
    let output = CellOutput::new_builder()
        .lock(receiver)
        .capacity(capacity.pack())
        .build();
    let builder = CapacityTransferBuilder::new(vec![(output, Bytes::default())]);
    let (tx, still_locked_groups) = builder.build_unlocked(
        &mut cell_collector,
        &cell_dep_resolver,
        &header_dep_resolver,
        &tx_dep_provider,
        &balancer,
        &unlockers,
    )?;
    assert!(still_locked_groups.is_empty());
    Ok(tx)
}

pub fn get_signer(
    from_address: Option<Address>,
    from_key: Option<H256>,
) -> Result<(Script, Box<dyn Signer>), Error> {
    let from_key = from_key
        .map(|data| {
            secp256k1::SecretKey::from_slice(data.as_bytes())
                .map_err(|err| anyhow!("invalid from key: {}", err))
        })
        .transpose()?;
    if let Some(privkey) = from_key {
        let sender = {
            let pubkey = secp256k1::PublicKey::from_secret_key(&SECP256K1, &privkey);
            let hash160 = blake2b_256(&pubkey.serialize()[..])[0..20].to_vec();
            Script::new_builder()
                .code_hash(SIGHASH_TYPE_HASH.pack())
                .hash_type(ScriptHashType::Type.into())
                .args(Bytes::from(hash160).pack())
                .build()
        };
        let signer = SecpCkbRawKeySigner::new_with_secret_keys(vec![privkey]);
        Ok((sender, Box::new(signer) as Box<_>))
    } else {
        let from_address = from_address.expect("from address");
        let sender = Script::from(&from_address);
        if sender.code_hash().as_slice() != SIGHASH_TYPE_HASH.as_bytes()
            || sender.hash_type().as_slice() != [ScriptHashType::Type as u8]
            || sender.args().raw_data().len() != 20
        {
            return Err(anyhow!("from address is not sighash address"));
        }
        let account = H160::from_slice(sender.args().raw_data().as_ref()).unwrap();
        let pass = prompt_password("Password: ")?;
        let signer = FileSystemKeystoreSigner::new(get_keystore()?);
        signer.unlock(&account, pass.as_bytes())?;
        Ok((sender, Box::new(signer) as Box<_>))
    }
}

fn get_keystore() -> Result<KeyStore, Error> {
    let ckb_cli_dir = if let Ok(dir) = env::var("CKB_CLI_HOME") {
        dir
    } else if let Ok(home) = env::var("HOME") {
        format!("{}/.ckb-cli", home)
    } else {
        return Err(anyhow!(
            "CKB_CLI_HOME and HOME environment variables not set"
        ));
    };
    let mut keystore_dir = PathBuf::from(ckb_cli_dir);
    keystore_dir.push("keystore");
    Ok(KeyStore::from_dir(keystore_dir, ScryptType::default())?)
}
