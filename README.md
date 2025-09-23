# wrapless-lightning-node
Implementation of Lightning node that utilises [Wrapless protocol](https://arxiv.org/abs/2507.06064) logic. 

## Installation
```
git clone https://github.com/distributed-lab/wrapless-lightning-node.git
```

## Usage
```
cd wrapless-lightning-node
cargo run <bitcoind-rpc-username>:<bitcoind-rpc-password>@<bitcoind-rpc-host>:<bitcoind-rpc-port> <ldk_storage_directory_path> [<ldk-peer-listening-port>] [<bitcoin-network>] [<announced-node-name>] [<announced-listen-addr>]
```
`bitcoind`'s RPC username and password likely can be found through `cat ~/.bitcoin/.cookie`.

`bitcoin-network`: defaults to `testnet`. Options: `testnet`, `regtest`, and `signet`.

`ldk-peer-listening-port`: defaults to 9735.

`announced-listen-addr` and `announced-node-name`: default to nothing, disabling any public announcements of this node.
`announced-listen-addr` can be set to an IPv4 or IPv6 address to announce that as a publicly-connectable address for this node.
`announced-node-name` can be any string up to 32 bytes in length, representing this node's alias.
