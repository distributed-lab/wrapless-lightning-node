use std::str::{FromStr, SplitWhitespace};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use bitcoin::hashes::Hash;
use lightning::bolt11_invoice::Bolt11Invoice;
use lightning::io::Write;
use lightning::ln::bolt11_payment::{payment_parameters_from_invoice, payment_parameters_from_variable_amount_invoice};
use lightning::ln::channelmanager::{PaymentId, Retry};
use lightning::offers::offer;
use lightning::offers::offer::Offer;
use lightning::onion_message::dns_resolution::HumanReadableName;
use lightning::onion_message::messenger::Destination;
use lightning::sign::{EntropySource, KeysManager};
use lightning::util::persist::KVStore;
use lightning::util::ser::Writeable;
use lightning_persister::fs_store::FilesystemStore;
use ldk::common::{ChannelManager, HTLCStatus, MillisatAmount, NetworkGraph, OutboundPaymentInfoStorage, PaymentInfo};
use ldk::disk::OUTBOUND_PAYMENTS_FNAME;

pub (crate) fn send_payment_cli(mut words: SplitWhitespace, keys_manager: &Arc<KeysManager>, outbound_payments: &Arc<Mutex<OutboundPaymentInfoStorage>>,
                                channel_manager: &Arc<ChannelManager>, mut line: String, fs_store: &Arc<FilesystemStore>,
                                network_graph: &Arc<NetworkGraph>) {
    let invoice_str = words.next();
    if invoice_str.is_none() {
        println!("ERROR: sendpayment requires an invoice: `sendpayment <invoice> [amount_msat]`");
        return;
    }
    let invoice_str = invoice_str.unwrap();

    let mut user_provided_amt: Option<u64> = None;
    if let Some(amt_msat_str) = words.next() {
        match amt_msat_str.parse() {
            Ok(amt) => user_provided_amt = Some(amt),
            Err(e) => {
                println!("ERROR: couldn't parse amount_msat: {}", e);
                return;
            },
        };
    }

    if let Ok(offer) = Offer::from_str(invoice_str) {
        let random_bytes = keys_manager.get_secure_random_bytes();
        let payment_id = PaymentId(random_bytes);

        let amt_msat = match (offer.amount(), user_provided_amt) {
            (Some(offer::Amount::Bitcoin { amount_msats }), _) => amount_msats,
            (_, Some(amt)) => amt,
            (amt, _) => {
                println!("ERROR: Cannot process non-Bitcoin-denominated offer value {:?}", amt);
                return;
            },
        };
        if user_provided_amt.is_some() && user_provided_amt != Some(amt_msat) {
            println!("Amount didn't match offer of {}msat", amt_msat);
            return;
        }

        while user_provided_amt.is_none() {
            print!("Paying offer for {} msat. Continue (Y/N)? >", amt_msat);
            std::io::stdout().flush().unwrap();

            if let Err(e) = std::io::stdin().read_line(&mut line) {
                println!("ERROR: {}", e);
                return;
            }

            if line.len() == 0 {
                // We hit EOF / Ctrl-D
                return;
            }

            if line.starts_with("Y") {
                break;
            }
            if line.starts_with("N") {
                return;
            }
        }

        outbound_payments.lock().unwrap().payments.insert(
            payment_id,
            PaymentInfo {
                preimage: None,
                secret: None,
                status: HTLCStatus::Pending,
                amt_msat: MillisatAmount(Some(amt_msat)),
            },
        );
        fs_store
            .write("", "", OUTBOUND_PAYMENTS_FNAME, &outbound_payments.encode())
            .unwrap();

        let retry = Retry::Timeout(Duration::from_secs(10));
        let amt = Some(amt_msat);
        let pay = channel_manager
            .pay_for_offer(&offer, None, amt, None, payment_id, retry, None);
        if pay.is_ok() {
            println!("Payment in flight");
        } else {
            println!("ERROR: Failed to pay: {:?}", pay);
        }
    } else if let Ok(hrn) = HumanReadableName::from_encoded(invoice_str) {
        let random_bytes = keys_manager.get_secure_random_bytes();
        let payment_id = PaymentId(random_bytes);

        if user_provided_amt.is_none() {
            println!("Can't pay to a human-readable-name without an amount");
            return;
        }

        // We need some nodes that will resolve DNS for us in order to pay a Human
        // Readable Name. They don't need to be trusted, but until onion message
        // forwarding is widespread we'll directly connect to them, revealing who
        // we intend to pay.
        let mut dns_resolvers = Vec::new();
        for (node_id, node) in network_graph.read_only().nodes().unordered_iter() {
            if let Some(info) = &node.announcement_info {
                // Sadly, 31 nodes currently squat on the DNS Resolver feature bit
                // without speaking it.
                // Its unclear why they're doing so, but none of them currently
                // also have the onion messaging feature bit set, so here we check
                // for both.
                let supports_dns = info.features().supports_dns_resolution();
                let supports_om = info.features().supports_onion_messages();
                if supports_dns && supports_om {
                    if let Ok(pubkey) = node_id.as_pubkey() {
                        dns_resolvers.push(Destination::Node(pubkey));
                    }
                }
            }
            if dns_resolvers.len() > 5 {
                break;
            }
        }
        if dns_resolvers.is_empty() {
            println!(
                "Failed to find any DNS resolving nodes, check your network graph is synced"
            );
            return;
        }

        let amt_msat = user_provided_amt.unwrap();
        outbound_payments.lock().unwrap().payments.insert(
            payment_id,
            PaymentInfo {
                preimage: None,
                secret: None,
                status: HTLCStatus::Pending,
                amt_msat: MillisatAmount(Some(amt_msat)),
            },
        );
        fs_store
            .write("", "", OUTBOUND_PAYMENTS_FNAME, &outbound_payments.encode())
            .unwrap();

        let retry = Retry::Timeout(Duration::from_secs(10));
        let pay = |a, b, c, d, e, f| {
            channel_manager.pay_for_offer_from_human_readable_name(a, b, c, d, e, f)
        };
        let pay = pay(hrn, amt_msat, payment_id, retry, None, dns_resolvers);
        if pay.is_ok() {
            println!("Payment in flight");
        } else {
            println!("ERROR: Failed to pay");
        }
    } else {
        match Bolt11Invoice::from_str(invoice_str) {
            Ok(invoice) => send_payment(
                &channel_manager,
                &invoice,
                user_provided_amt,
                &mut outbound_payments.lock().unwrap(),
                Arc::clone(&fs_store),
            ),
            Err(e) => {
                println!("ERROR: invalid invoice: {:?}", e);
            },
        }
    }
}

fn send_payment(
    channel_manager: &ChannelManager, invoice: &Bolt11Invoice, required_amount_msat: Option<u64>,
    outbound_payments: &mut OutboundPaymentInfoStorage, fs_store: Arc<FilesystemStore>,
) {
    let payment_id = PaymentId((*invoice.payment_hash()).to_byte_array());
    let payment_secret = Some(*invoice.payment_secret());
    let zero_amt_invoice =
        invoice.amount_milli_satoshis().is_none() || invoice.amount_milli_satoshis() == Some(0);
    let pay_params_opt = if zero_amt_invoice {
        if let Some(amt_msat) = required_amount_msat {
            payment_parameters_from_variable_amount_invoice(invoice, amt_msat)
        } else {
            println!("Need an amount for the given 0-value invoice");
            print!("> ");
            return;
        }
    } else {
        if required_amount_msat.is_some() && invoice.amount_milli_satoshis() != required_amount_msat
        {
            println!(
                "Amount didn't match invoice value of {}msat",
                invoice.amount_milli_satoshis().unwrap_or(0)
            );
            print!("> ");
            return;
        }
        payment_parameters_from_invoice(invoice)
    };
    let (payment_hash, recipient_onion, route_params) = match pay_params_opt {
        Ok(res) => res,
        Err(e) => {
            println!("Failed to parse invoice: {:?}", e);
            print!("> ");
            return;
        },
    };
    outbound_payments.payments.insert(
        payment_id,
        PaymentInfo {
            preimage: None,
            secret: payment_secret,
            status: HTLCStatus::Pending,
            amt_msat: MillisatAmount(invoice.amount_milli_satoshis()),
        },
    );
    fs_store.write("", "", OUTBOUND_PAYMENTS_FNAME, &outbound_payments.encode()).unwrap();

    match channel_manager.send_payment(
        payment_hash,
        recipient_onion,
        payment_id,
        route_params,
        Retry::Timeout(Duration::from_secs(10)),
    ) {
        Ok(_) => {
            let payee_pubkey = invoice.recover_payee_pub_key();
            let amt_msat = invoice.amount_milli_satoshis().unwrap();
            println!("EVENT: initiated sending {} msats to {}", amt_msat, payee_pubkey);
            print!("> ");
        },
        Err(e) => {
            println!("ERROR: failed to send payment: {:?}", e);
            print!("> ");
            outbound_payments.payments.get_mut(&payment_id).unwrap().status = HTLCStatus::Failed;
            fs_store.write("", "", OUTBOUND_PAYMENTS_FNAME, &outbound_payments.encode()).unwrap();
        },
    };
}