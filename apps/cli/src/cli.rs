use ldk::common::{
	ChainMonitor, ChannelManager, HTLCStatus, InboundPaymentInfoStorage, MillisatAmount,
	NetworkGraph, OutboundPaymentInfoStorage, PaymentInfo, PeerManager,
};
use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::hashes::Hash;
use bitcoin::network::Network;
use bitcoin::secp256k1::PublicKey;
use std::env;
use std::io::Write;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use lightning::ln::msgs::SocketAddress;
use lightning::ln::types::ChannelId;
use lightning::routing::gossip::NodeId;
use lightning::sign::KeysManager;
use lightning_persister::fs_store::FilesystemStore;
use rustyline::DefaultEditor;
use crate::get_invoice::{get_invoice_cli};
use crate::nodeinfo::node_info_cli;
use crate::open_channel::open_channel_cli;
use crate::send_payment::send_payment_cli;
use crate::close_channel::close_channel_cli;
use crate::force_close_channel::force_close_channel_cli;

pub(crate) struct LdkUserInfo {
	pub(crate) bitcoind_rpc_username: String,
	pub(crate) bitcoind_rpc_password: String,
	pub(crate) bitcoind_rpc_port: u16,
	pub(crate) bitcoind_rpc_host: String,
	pub(crate) ldk_storage_dir_path: String,
	pub(crate) ldk_peer_listening_port: u16,
	pub(crate) ldk_announced_listen_addr: Vec<SocketAddress>,
	pub(crate) ldk_announced_node_name: [u8; 32],
	pub(crate) network: Network,
}

/// Asks for user input in terms of a Wrapless protocol.
pub(crate) fn poll_for_user_input_wrapless(
    peer_manager: Arc<PeerManager>, channel_manager: Arc<ChannelManager>,
    chain_monitor: Arc<ChainMonitor>, keys_manager: Arc<KeysManager>,
    network_graph: Arc<NetworkGraph>, inbound_payments: Arc<Mutex<InboundPaymentInfoStorage>>,
    outbound_payments: Arc<Mutex<OutboundPaymentInfoStorage>>, ldk_data_dir: String,
    fs_store: Arc<FilesystemStore>,
) {
    let mut rl = DefaultEditor::new().unwrap();

    println!(
        "Wrapless LDK startup successful. Enter \"help\" to view available commands. Press Ctrl-D to quit."
    );
    println!("LDK logs are available at <your-supplied-ldk-data-dir-path>/.ldk/logs");
    println!("Local Node ID is {}.", channel_manager.get_our_node_id());
    'read_command: loop {
        let line = if let Ok(line) = rl.readline("> ") {
            rl.add_history_entry(line.clone()).unwrap();
            line
        } else {
            break println!("ERROR");
        };

        if line.len() == 0 {
            // We hit EOF / Ctrl-D
            break;
        }

        let mut words = line.split_whitespace();
        if let Some(word) = words.next() {
            match word {
                "help" => help(),
                "openchannel" => open_channel_cli(words, &peer_manager, &channel_manager, &ldk_data_dir),
                "getinvoice" => get_invoice_cli(words, &inbound_payments, &fs_store, &channel_manager),
                "nodeinfo" => node_info_cli(&channel_manager, &chain_monitor, &peer_manager, &network_graph),
                "sendpayment" => send_payment_cli(words, &keys_manager, &outbound_payments, &channel_manager,
                line.clone(), &fs_store, &network_graph),
                "listchannels" => list_channels(&channel_manager, &network_graph),
                "closechannel" => close_channel_cli(words, &channel_manager),
                "forceclosechannel" => force_close_channel_cli(words, &channel_manager),
                "quit" | "exit" => break,
                _ => println!("Unknown command. See `\"help\" for available commands."),
            }
        }
    }
}

pub(crate) fn help() {
    let package_version = env!("CARGO_PKG_VERSION");
    let package_name = env!("CARGO_PKG_NAME");
    println!("\nVERSION:");
    println!("  {} v{}", package_name, package_version);
    println!("\nUSAGE:");
    println!("  Command [arguments]");
    println!("\nCOMMANDS:");
    println!("  help\tShows a list of commands.");
    println!("  quit\tClose the application.");
    println!("\n  Channels:");
    println!("      openchannel pubkey@[host:port] <amt_satoshis> [--public] [--with-anchors]");
    println!("      closechannel <channel_id> <peer_pubkey>");
    println!("      forceclosechannel <channel_id> <peer_pubkey>");
    println!("      listchannels");
    println!("\n  Peers:");
    // println!("      connectpeer pubkey@host:port");
    // println!("      disconnectpeer <peer_pubkey>");
    // println!("      listpeers");
    println!("\n  Payments:");
    println!("      sendpayment <invoice|offer|human readable name> [<amount_msat>]");
    // println!("      keysend <dest_pubkey> <amt_msats>");
    // println!("      listpayments");
    println!("\n  Invoices:");
    println!("      getinvoice <amt_msats> <expiry_secs>");
    // println!("      getoffer [<amt_msats>]");
    println!("\n  Other:");
    // println!("      signmessage <message>");
    println!("      nodeinfo");
}

pub(crate) async fn do_connect_peer(
    pubkey: PublicKey, peer_addr: SocketAddr, peer_manager: Arc<PeerManager>,
) -> Result<(), ()> {
    match lightning_net_tokio::connect_outbound(Arc::clone(&peer_manager), pubkey, peer_addr).await
    {
        Some(connection_closed_future) => {
            let mut connection_closed_future = Box::pin(connection_closed_future);
            loop {
                tokio::select! {
					_ = &mut connection_closed_future => return Err(()),
					_ = tokio::time::sleep(Duration::from_millis(10)) => {},
				};
                if peer_manager.peer_by_node_id(&pubkey).is_some() {
                    return Ok(());
                }
            }
        },
        None => Err(()),
    }
}

pub(crate) async fn connect_peer_if_necessary(
    pubkey: PublicKey, peer_addr: SocketAddr, peer_manager: Arc<PeerManager>,
) -> Result<(), ()> {
    for peer_details in peer_manager.list_peers() {
        if peer_details.counterparty_node_id == pubkey {
            return Ok(());
        }
    }
    let res = do_connect_peer(pubkey, peer_addr, peer_manager).await;
    if res.is_err() {
        println!("ERROR: failed to connect to peer");
    }
    res
}

fn list_channels(channel_manager: &Arc<ChannelManager>, network_graph: &Arc<NetworkGraph>) {
    print!("[");
    for chan_info in channel_manager.list_channels() {
        println!("");
        println!("\t{{");
        println!("\t\tchannel_id: {},", chan_info.channel_id);
        if let Some(funding_txo) = chan_info.funding_txo {
            println!("\t\tfunding_txid: {},", funding_txo.txid);
        }

        println!(
            "\t\tpeer_pubkey: {},",
            ldk::hex_utils::hex_str(&chan_info.counterparty.node_id.serialize())
        );
        if let Some(node_info) = network_graph
            .read_only()
            .nodes()
            .get(&NodeId::from_pubkey(&chan_info.counterparty.node_id))
        {
            if let Some(announcement) = &node_info.announcement_info {
                println!("\t\tpeer_alias: {}", announcement.alias());
            }
        }

        if let Some(id) = chan_info.short_channel_id {
            println!("\t\tshort_channel_id: {},", id);
        }
        println!("\t\tis_channel_ready: {},", chan_info.is_channel_ready);
        println!("\t\tchannel_value_satoshis: {},", chan_info.channel_value_satoshis);
        println!("\t\toutbound_capacity_msat: {},", chan_info.outbound_capacity_msat);
        if chan_info.is_usable {
            println!("\t\tavailable_balance_for_send_msat: {},", chan_info.outbound_capacity_msat);
            println!("\t\tavailable_balance_for_recv_msat: {},", chan_info.inbound_capacity_msat);
        }
        println!("\t\tchannel_can_send_payments: {},", chan_info.is_usable);
        println!("\t\tpublic: {},", chan_info.is_announced);
        println!("\t}},");
    }
    println!("]");
}
