use std::str::SplitWhitespace;
use std::sync::Arc;
use bitcoin::secp256k1::PublicKey;
use lightning::ln::types::ChannelId;
use ldk::common::ChannelManager;
use ldk::hex_utils;

pub(crate) fn force_close_channel_cli(mut words: SplitWhitespace, channel_manager: &Arc<ChannelManager>) {
    let channel_id_str = words.next();
    if channel_id_str.is_none() {
        println!("ERROR: forceclosechannel requires a channel ID: `forceclosechannel <channel_id> <peer_pubkey>`");
        return;
    }
    let channel_id_vec = hex_utils::to_vec(channel_id_str.unwrap());
    if channel_id_vec.is_none() || channel_id_vec.as_ref().unwrap().len() != 32 {
        println!("ERROR: couldn't parse channel_id");
        return;
    }
    let mut channel_id = [0; 32];
    channel_id.copy_from_slice(&channel_id_vec.unwrap());

    let peer_pubkey_str = words.next();
    if peer_pubkey_str.is_none() {
        println!("ERROR: forceclosechannel requires a peer pubkey: `forceclosechannel <channel_id> <peer_pubkey>`");
        return;
    }
    let peer_pubkey_vec = match hex_utils::to_vec(peer_pubkey_str.unwrap()) {
        Some(peer_pubkey_vec) => peer_pubkey_vec,
        None => {
            println!("ERROR: couldn't parse peer_pubkey");
            return;
        },
    };
    let peer_pubkey = match PublicKey::from_slice(&peer_pubkey_vec) {
        Ok(peer_pubkey) => peer_pubkey,
        Err(_) => {
            println!("ERROR: couldn't parse peer_pubkey");
            return;
        },
    };

    force_close_channel(channel_id, peer_pubkey, channel_manager.clone());
}

fn force_close_channel(
    channel_id: [u8; 32], counterparty_node_id: PublicKey, channel_manager: Arc<ChannelManager>,
) {
    match channel_manager.force_close_broadcasting_latest_txn(
        &ChannelId(channel_id),
        &counterparty_node_id,
        "Manually force-closed".to_string(),
    ) {
        Ok(()) => println!("EVENT: initiating channel force-close"),
        Err(e) => println!("ERROR: failed to force-close channel: {:?}", e),
    }
}