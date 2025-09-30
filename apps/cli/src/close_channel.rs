use std::str::SplitWhitespace;
use std::sync::Arc;
use bitcoin::secp256k1::PublicKey;
use lightning::ln::types::ChannelId;
use ldk::common::ChannelManager;

pub(crate) fn close_channel_cli(mut words: SplitWhitespace, channel_manager: &Arc<ChannelManager>) {
    let channel_id_str = words.next();
    if channel_id_str.is_none() {
        println!("ERROR: closechannel requires a channel ID: `closechannel <channel_id> <peer_pubkey>`");
        return;
    }
    let channel_id_vec = ldk::hex_utils::to_vec(channel_id_str.unwrap());
    if channel_id_vec.is_none() || channel_id_vec.as_ref().unwrap().len() != 32 {
        println!("ERROR: couldn't parse channel_id");
        return;
    }
    let mut channel_id = [0; 32];
    channel_id.copy_from_slice(&channel_id_vec.unwrap());

    let peer_pubkey_str = words.next();
    if peer_pubkey_str.is_none() {
        println!("ERROR: closechannel requires a peer pubkey: `closechannel <channel_id> <peer_pubkey>`");
        return;
    }
    let peer_pubkey_vec = match ldk::hex_utils::to_vec(peer_pubkey_str.unwrap()) {
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

    close_channel(channel_id, peer_pubkey, channel_manager.clone());
}

pub(crate) fn close_channel(
    channel_id: [u8; 32], counterparty_node_id: PublicKey, channel_manager: Arc<ChannelManager>,
) {
    match channel_manager.close_channel(&ChannelId(channel_id), &counterparty_node_id) {
        Ok(()) => println!("EVENT: initiating channel close"),
        Err(e) => println!("ERROR: failed to close channel: {:?}", e),
    }
}
