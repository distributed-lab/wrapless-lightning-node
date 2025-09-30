use std::str::SplitWhitespace;
use std::sync::{Arc, Mutex};
use bitcoin::hashes::Hash;
use lightning::ln::channelmanager::Bolt11InvoiceParameters;
use lightning::types::payment::PaymentHash;
use lightning::util::persist::KVStore;
use lightning::util::ser::Writeable;
use lightning_persister::fs_store::FilesystemStore;
use ldk::common::{ChannelManager, HTLCStatus, InboundPaymentInfoStorage, MillisatAmount, PaymentInfo};
use ldk::disk::INBOUND_PAYMENTS_FNAME;

pub(crate) fn get_invoice_cli(mut words: SplitWhitespace, inbound_payments: &Arc<Mutex<InboundPaymentInfoStorage>>, fs_store: &Arc<FilesystemStore>, channel_manager: &Arc<ChannelManager>) {
    let amt_str = words.next();
    if amt_str.is_none() {
        println!("ERROR: getinvoice requires an amount in millisatoshis");
        return;
    }

    let amt_msat: Result<u64, _> = amt_str.unwrap().parse();
    if amt_msat.is_err() {
        println!("ERROR: getinvoice provided payment amount was not a number");
        return;
    }

    let expiry_secs_str = words.next();
    if expiry_secs_str.is_none() {
        println!("ERROR: getinvoice requires an expiry in seconds");
        return;
    }

    let expiry_secs: Result<u32, _> = expiry_secs_str.unwrap().parse();
    if expiry_secs.is_err() {
        println!("ERROR: getinvoice provided expiry was not a number");
        return;
    }

    let mut inbound_payments = inbound_payments.lock().unwrap();
    get_invoice(
        amt_msat.unwrap(),
        &mut inbound_payments,
        &channel_manager,
        expiry_secs.unwrap(),
    );
    fs_store
        .write("", "", INBOUND_PAYMENTS_FNAME, &inbound_payments.encode())
        .unwrap();
}

fn get_invoice(
    amt_msat: u64, inbound_payments: &mut InboundPaymentInfoStorage,
    channel_manager: &ChannelManager, expiry_secs: u32,
) {
    let mut invoice_params: Bolt11InvoiceParameters = Default::default();
    invoice_params.amount_msats = Some(amt_msat);
    invoice_params.invoice_expiry_delta_secs = Some(expiry_secs);
    let invoice = match channel_manager.create_bolt11_invoice(invoice_params) {
        Ok(inv) => {
            println!("SUCCESS: generated invoice: {}", inv);
            inv
        },
        Err(e) => {
            println!("ERROR: failed to create invoice: {:?}", e);
            return;
        },
    };

    let payment_hash = PaymentHash(invoice.payment_hash().to_byte_array());
    inbound_payments.payments.insert(
        payment_hash,
        PaymentInfo {
            preimage: None,
            secret: Some(invoice.payment_secret().clone()),
            status: HTLCStatus::Pending,
            amt_msat: MillisatAmount(Some(amt_msat)),
        },
    );
}
