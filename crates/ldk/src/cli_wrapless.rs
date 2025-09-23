// use crate::{
//     ChainMonitor, ChannelManager, InboundPaymentInfoStorage,
//     NetworkGraph, OutboundPaymentInfoStorage, PeerManager,
// };
// use lightning::sign::{KeysManager};
// use lightning_persister::fs_store::FilesystemStore;
// use std::sync::{Arc, Mutex};
// use rustyline::DefaultEditor;
// 
// /// Asks for user input in terms of a Wrapless protocol.
// pub(crate) fn poll_for_user_input_wrapless(
//     peer_manager: Arc<PeerManager>, channel_manager: Arc<ChannelManager>,
//     chain_monitor: Arc<ChainMonitor>, keys_manager: Arc<KeysManager>,
//     network_graph: Arc<NetworkGraph>, inbound_payments: Arc<Mutex<InboundPaymentInfoStorage>>,
//     outbound_payments: Arc<Mutex<OutboundPaymentInfoStorage>>, ldk_data_dir: String,
//     fs_store: Arc<FilesystemStore>,
// ) {
//     let mut rl = DefaultEditor::new().unwrap();
// 
//     println!(
//         "Wrapless LDK startup successful. Enter \"help\" to view available commands. Press Ctrl-D to quit."
//     );
//     println!("LDK logs are available at <your-supplied-ldk-data-dir-path>/.ldk/logs");
//     println!("Local Node ID is {}.", channel_manager.get_our_node_id());
//     'read_command: loop {
//         let line = if let Ok(line) = rl.readline("> ") {
//             rl.add_history_entry(line.clone()).unwrap();
//             line
//         } else {
//             break println!("ERROR");
//         };
// 
//         if line.len() == 0 {
//             // We hit EOF / Ctrl-D
//             break;
//         }
// 
//         let mut words = line.split_whitespace();
//         if let Some(word) = words.next() {
//             match word {
//                 "help" => help(),
//                 "quit" | "exit" => break,
//                 _ => println!("Unknown command. See `\"help\" for available commands."),
//             }
//         }
//     }
// }
// 
// 
// pub(crate) fn help() {
//     let package_version = env!("CARGO_PKG_VERSION");
//     let package_name = env!("CARGO_PKG_NAME");
//     println!("\nVERSION:");
//     println!("  {} v{}", package_name, package_version);
//     println!("\nUSAGE:");
//     println!("  Command [arguments]");
//     println!("\nCOMMANDS:");
//     println!("  help\tShows a list of commands.");
//     println!("  quit\tClose the application.");
//     println!("\n  Channels:");
//     // println!("      openchannel pubkey@[host:port] <amt_satoshis> [--public] [--with-anchors]");
//     // println!("      closechannel <channel_id> <peer_pubkey>");
//     // println!("      forceclosechannel <channel_id> <peer_pubkey>");
//     // println!("      listchannels");
//     println!("\n  Peers:");
//     // println!("      connectpeer pubkey@host:port");
//     // println!("      disconnectpeer <peer_pubkey>");
//     // println!("      listpeers");
//     println!("\n  Payments:");
//     // println!("      sendpayment <invoice|offer|human readable name> [<amount_msat>]");
//     // println!("      keysend <dest_pubkey> <amt_msats>");
//     // println!("      listpayments");
//     println!("\n  Invoices:");
//     // println!("      getinvoice <amt_msats> <expiry_secs>");
//     // println!("      getoffer [<amt_msats>]");
//     println!("\n  Other:");
//     // println!("      signmessage <message>");
//     // println!("      nodeinfo");
// }