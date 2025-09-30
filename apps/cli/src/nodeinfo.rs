use std::sync::Arc;
use lightning::chain::channelmonitor::Balance;
use ldk::common::{ChainMonitor, ChannelManager, NetworkGraph, PeerManager};

pub(crate) fn node_info_cli(
    channel_manager: &Arc<ChannelManager>, chain_monitor: &Arc<ChainMonitor>,
    peer_manager: &Arc<PeerManager>, network_graph: &Arc<NetworkGraph>,
) {
    println!("\t{{");
    println!("\t\t node_pubkey: {}", channel_manager.get_our_node_id());
    let chans = channel_manager.list_channels();
    println!("\t\t num_channels: {}", chans.len());
    println!("\t\t num_usable_channels: {}", chans.iter().filter(|c| c.is_usable).count());
    let balances = chain_monitor.get_claimable_balances(&[]);
    let local_balance_sat = balances.iter().map(|b| b.claimable_amount_satoshis()).sum::<u64>();
    println!("\t\t local_balance_sats: {}", local_balance_sat);
    let close_fees_map = |b| match b {
        &Balance::ClaimableOnChannelClose { transaction_fee_satoshis, .. } => {
            transaction_fee_satoshis
        },
        _ => 0,
    };
    let close_fees_sats = balances.iter().map(close_fees_map).sum::<u64>();
    println!("\t\t eventual_close_fees_sats: {}", close_fees_sats);
    let pending_payments_map = |b| match b {
        &Balance::MaybeTimeoutClaimableHTLC { amount_satoshis, outbound_payment, .. } => {
            if outbound_payment {
                amount_satoshis
            } else {
                0
            }
        },
        _ => 0,
    };
    let pending_payments = balances.iter().map(pending_payments_map).sum::<u64>();
    println!("\t\t pending_outbound_payments_sats: {}", pending_payments);
    println!("\t\t num_peers: {}", peer_manager.list_peers().len());
    let graph_lock = network_graph.read_only();
    println!("\t\t network_nodes: {}", graph_lock.nodes().len());
    println!("\t\t network_channels: {}", graph_lock.channels().len());
    println!("\t}},");
}