use anyhow::{anyhow, Error};
use ckb_hash::blake2b_256;
use ckb_sdk::{
    constants::{MULTISIG_TYPE_HASH, SIGHASH_TYPE_HASH},
    rpc::{
        ckb_light_client::{ScriptType, SearchKey},
        LightClientRpcClient,
    },
    traits::{
        DefaultCellDepResolver, LightClientCellCollector, LightClientHeaderDepResolver,
        LightClientTransactionDependencyProvider, SecpCkbRawKeySigner,
    },
    tx_builder::{transfer::CapacityTransferBuilder, CapacityBalancer, TxBuilder},
    unlock::{ScriptUnlocker, SecpSighashUnlocker},
    Address, ScriptId, SECP256K1,
};
use ckb_types::{
    bytes::Bytes,
    core::{ScriptHashType, TransactionView},
    packed::{CellOutput, Script, WitnessArgs},
    prelude::*,
    H256,
};
use std::collections::HashMap;

pub fn get_capacity(rpc_url: &str, address: Address) -> Result<u64, Error> {
    let mut client = LightClientRpcClient::new(rpc_url);
    let script = Script::from(&address).into();
    if !client
        .get_scripts()?
        .iter()
        .any(|status| status.script == script)
    {
        println!("[NOTE]: address not registered, you may use `rpc set-scripts` subcommand to register the address");
    }
    let search_key = SearchKey {
        script,
        script_type: ScriptType::Lock,
        filter: None,
        group_by_transaction: None,
    };
    let capacity: u64 = client.get_cells_capacity(search_key)?.value();
    Ok(capacity)
}

pub fn build_transfer_tx(
    rpc_url: &str,
    from_address: Option<Address>,
    from_key: Option<secp256k1::SecretKey>,
    to_address: Address,
    capacity: u64,
    skip_check_to_address: bool,
) -> Result<TransactionView, Error> {
    let from_key = from_key.ok_or_else(|| anyhow!("from key is missing"))?;
    let sender = {
        let pubkey = secp256k1::PublicKey::from_secret_key(&SECP256K1, &from_key);
        let hash160 = blake2b_256(&pubkey.serialize()[..])[0..20].to_vec();
        Script::new_builder()
            .code_hash(SIGHASH_TYPE_HASH.pack())
            .hash_type(ScriptHashType::Type.into())
            .args(Bytes::from(hash160).pack())
            .build()
    };

    let signer = SecpCkbRawKeySigner::new_with_secret_keys(vec![from_key]);
    let sighash_unlocker = SecpSighashUnlocker::from(Box::new(signer) as Box<_>);
    let sighash_script_id = ScriptId::new_type(SIGHASH_TYPE_HASH.clone());
    let mut unlockers = HashMap::default();
    unlockers.insert(
        sighash_script_id,
        Box::new(sighash_unlocker) as Box<dyn ScriptUnlocker>,
    );

    // Build CapacityBalancer
    let placeholder_witness = WitnessArgs::new_builder()
        .lock(Some(Bytes::from(vec![0u8; 65])).pack())
        .build();
    let balancer = CapacityBalancer::new_simple(sender, placeholder_witness, 1000);

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
