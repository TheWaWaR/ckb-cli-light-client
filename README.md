# Light Client Command Line Tool

## All sub-commands
```
$ ckb-cli-light-client --help
Usage: ckb-cli-light-client [OPTIONS] <COMMAND>

Commands:
  get-capacity        Get capacity of an address
  transfer            Transfer some capacity from given address to a receiver address
  dao                 Nervos DAO operations
  example-search-key  Output the example `SearchKey` value
  rpc                 Send jsonrpc call the ckb-light-client rpc server
  help                Print this message or the help of the given subcommand(s)

Options:
      --rpc <URL>  CKB light client rpc url [default: http://127.0.0.1:9000]
      --debug      Debug mode, print more information
```

# Tutorial

## Prepare for the tutorial

Firstly, you need start a [ckb-light-client][ckb-light-client-repo] program. The default rpc address is `http://127.0.0.1:9000`.

If you want use a key from [ckb-cli][ckb-cli-repo], you can use follow command to create an account:
```
$ ckb-cli account new
Your new account is locked with a password. Please give a password. Do not forget this password.
Password:
Repeat password:
address:
  mainnet: ckb1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqt48tu9kdhtu6qa3rxyvth38ttmyk7ds7cg3e9w7
  testnet: ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqt48tu9kdhtu6qa3rxyvth38ttmyk7ds7cxrj2yx
address(deprecated):
  mainnet: ckb1qyq82whctvmwhe5pmzxvgch0zwkhkfdumpasqdhlw4
  testnet: ckt1qyq82whctvmwhe5pmzxvgch0zwkhkfdumpasagfqzf
lock_arg: 0x753af85b36ebe681d88cc462ef13ad7b25bcd87b
lock_hash: 0xb82e384482c41f010bc5decca782e8e8c6ad41bbcdac4187659fcb937526ab1e
```

Assume the ckb-light-client connected to CKB `testnet`, so the address `ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqt48tu9kdhtu6qa3rxyvth38ttmyk7ds7cxrj2yx` will be used. 

Then we need transfer some CKB to this address from other address, or get some CKB from [testnet faucet][testnet-faucet].

At last we install `ckb-cli-light-client` by `cargo install` (you may need install rust toolchain first).
```
$ cargo install --git https://github.com/TheWaWaR/ckb-cli-light-client.git --locked
```

## Transfer CKB capacity

Firstly, we query the capacity of the address:
```
$ ckb-cli-light-client get-capacity --address ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqt48tu9kdhtu6qa3rxyvth38ttmyk7ds7cxrj2yx
Error: address not registered, you may use `rpc set-scripts` subcommand to register the address
```

It says the address is not registered, we use `rpc set-scripts` to register it, so that ckb-light-client can index all the cells owner by this address:
```
$ ckb-cli-light-client rpc set-scripts --scripts ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqt48tu9kdhtu6qa3rxyvth38ttmyk7ds7cxrj2yx,1000
```

Then we query the capacity again:
```
$ ckb-cli-light-client get-capacity --address ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqt48tu9kdhtu6qa3rxyvth38ttmyk7ds7cxrj2yx
synchronized number: 7085789
tip number: 7085789
tip hash: 0x7777777777777777777777777777777777777777777777777777777777777777
capacity: 50000.0 CKB
```

Then, we better read the `transfer` sub-command help:
```
$ ckb-cli-light-client transfer --help
Transfer some capacity from given address to a receiver address

Usage: light-client transfer [OPTIONS] --to-address <ADDR> --capacity <CAPACITY> <--from-address <ADDR>|--from-key <PRIVKEY>>

Options:
      --from-address <ADDR>    The sender address (sighash only, also be used to match key in ckb-cli keystore)
      --from-key <PRIVKEY>     The sender private key (hex string, also be used to generate sighash address)
      --to-address <ADDR>      The receiver address
      --capacity <CAPACITY>    The capacity to transfer (unit: CKB, example: 102.43)
      --skip-check-to-address  Skip check <to-address> (default only allow sighash/multisig address), be cautious to use this flag
```

Transfer some CKB from `ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqt48tu9kdhtu6qa3rxyvth38ttmyk7ds7cxrj2yx` to an address:
```
$ ckb-cli-light-client transfer \
    --from-address ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqt48tu9kdhtu6qa3rxyvth38ttmyk7ds7cxrj2yx \
    --to-address ckt1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqgaqanf \
    --capcity 200.0
```
In above example, `ckb-cli-light-client` will search ckb-cli `keystore` directory by address (`ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqt48tu9kdhtu6qa3rxyvth38ttmyk7ds7cxrj2yx`) and use the key to sign the transaction. The `keystore` directory typically located in `~/.ckb-cli/keystore` or `$CKB_CLI_HOME/keystore` if you use the environment to specify the ckb-cli home directory.

Then we query the capacity of the address:
```
$ ckb-cli-light-client get-capacity --address ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqt48tu9kdhtu6qa3rxyvth38ttmyk7ds7cxrj2yx
synchronized number: 7085789
tip number: 7085789
tip hash: 0x8888888888888888888888888888888888888888888888888888888888888888
capacity: 49799.99996544 CKB
```

[ckb-light-client-repo]: https://github.com/nervosnetwork/ckb-light-client
[ckb-cli-repo]: https://github.com/nervosnetwork/ckb-cli
[testnet-faucet]: https://faucet.nervos.org/
