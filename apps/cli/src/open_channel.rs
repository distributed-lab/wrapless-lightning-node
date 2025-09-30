use std::path::Path;
use std::str::SplitWhitespace;
use std::sync::Arc;
use bitcoin::secp256k1::PublicKey;
use lightning::util::config::{ChannelHandshakeConfig, ChannelHandshakeLimits, UserConfig};
use ldk::common::{ChannelManager, PeerManager};
use crate::cli::connect_peer_if_necessary;
use crate::utils::parse_peer_info;

pub(crate) fn open_channel_cli(mut words: SplitWhitespace, peer_manager: &Arc<PeerManager>, channel_manager: &Arc<ChannelManager>, ldk_data_dir: &String) {
    let peer_pubkey_and_ip_addr = words.next();
    let channel_value_sat = words.next();
    if peer_pubkey_and_ip_addr.is_none() || channel_value_sat.is_none() {
        println!("ERROR: openchannel has 2 required arguments: `openchannel pubkey@host:port channel_amt_satoshis` [--public] [--with-anchors]");
        return;
    }
    let peer_pubkey_and_ip_addr = peer_pubkey_and_ip_addr.unwrap();

    let mut pubkey_and_addr = peer_pubkey_and_ip_addr.split("@");
    let pubkey = pubkey_and_addr.next();
    let peer_addr_str = pubkey_and_addr.next();
    let pubkey = ldk::hex_utils::to_compressed_pubkey(pubkey.unwrap());
    if pubkey.is_none() {
        println!("ERROR: unable to parse given pubkey for node");
        return;
    }
    let pubkey = pubkey.unwrap();

    if peer_addr_str.is_none() {
        if peer_manager.peer_by_node_id(&pubkey).is_none() {
            println!("ERROR: Peer address not provided and peer is not connected");
            return;
        }
    } else {
        let (pubkey, peer_addr) =
            match parse_peer_info(peer_pubkey_and_ip_addr.to_string()) {
                Ok(info) => info,
                Err(e) => {
                    println!("{:?}", e.into_inner().unwrap());
                    return;
                },
            };

        if tokio::runtime::Handle::current()
            .block_on(connect_peer_if_necessary(
                pubkey,
                peer_addr,
                peer_manager.clone(),
            ))
            .is_err()
        {
            return;
        };
    }

    let chan_amt_sat: Result<u64, _> = channel_value_sat.unwrap().parse();
    if chan_amt_sat.is_err() {
        println!("ERROR: channel amount must be a number");
        return;
    }
    let (mut announce_channel, mut with_anchors) = (false, false);
    while let Some(word) = words.next() {
        match word {
            "--public" | "--public=true" => announce_channel = true,
            "--public=false" => announce_channel = false,
            "--with-anchors" | "--with-anchors=true" => with_anchors = true,
            "--with-anchors=false" => with_anchors = false,
            _ => {
                println!("ERROR: invalid boolean flag format. Valid formats: `--option`, `--option=true` `--option=false`");
                continue;
            },
        }
    }

    if open_channel(
        pubkey,
        chan_amt_sat.unwrap(),
        announce_channel,
        with_anchors,
        channel_manager.clone(),
    )
        .is_ok()
    {
        if peer_addr_str.is_some() {
            let peer_data_path =
                format!("{}/channel_peer_data", ldk_data_dir.clone());
            let _ = ldk::disk::persist_channel_peer(
                Path::new(&peer_data_path),
                peer_pubkey_and_ip_addr,
            );
        }
    }
}

fn open_channel(
    peer_pubkey: PublicKey, channel_amt_sat: u64, announce_for_forwarding: bool,
    with_anchors: bool, channel_manager: Arc<ChannelManager>,
) -> Result<(), ()> {
    let config = UserConfig {
        channel_handshake_limits: ChannelHandshakeLimits {
            // lnd's max to_self_delay is 2016, so we want to be compatible.
            their_to_self_delay: 2016,
            ..Default::default()
        },
        channel_handshake_config: ChannelHandshakeConfig {
            announce_for_forwarding,
            negotiate_anchors_zero_fee_htlc_tx: with_anchors,
            ..Default::default()
        },
        ..Default::default()
    };

    match channel_manager.create_channel(peer_pubkey, channel_amt_sat, 0, 0, None, Some(config)) {
        Ok(_) => {
            println!("EVENT: initiated channel with peer {}. ", peer_pubkey);
            return Ok(());
        },
        Err(e) => {
            println!("ERROR: failed to open channel: {:?}", e);
            return Err(());
        },
    }
}
