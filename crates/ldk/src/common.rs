use crate::bitcoind_client::BitcoindClient;
use crate::disk::FilesystemLogger;
use bitcoin::io;
use lightning::chain::chainmonitor;
use lightning::chain::Filter;
use lightning::events::bump_transaction::{BumpTransactionEventHandler, Wallet};
use lightning::ln::channelmanager::{
    PaymentId, SimpleArcChannelManager,
};
use lightning::ln::msgs::DecodeError;
use lightning::ln::peer_handler::{
    IgnoringMessageHandler, PeerManager as LdkPeerManager,
};
use lightning::onion_message::messenger::{
    DefaultMessageRouter, OnionMessenger as LdkOnionMessenger,
};
use lightning::routing::gossip;
use lightning::routing::gossip::P2PGossipSync;
use lightning::sign::{InMemorySigner, KeysManager};
use lightning::types::payment::{PaymentHash, PaymentPreimage, PaymentSecret};
use lightning::util::hash_tables::HashMap;
use lightning::util::persist::MonitorUpdatingPersister;
use lightning::util::ser::{Readable, Writeable, Writer};
use lightning::util::sweep as ldk_sweep;
use lightning::{impl_writeable_tlv_based, impl_writeable_tlv_based_enum};
use lightning_dns_resolver::OMDomainResolver;
use lightning_net_tokio::SocketDescriptor;
use lightning_persister::fs_store::FilesystemStore;
use std::convert::TryInto;
use std::fmt;
use std::io::Write;
use std::sync::Arc;
#[derive(Copy, Clone)]
pub enum HTLCStatus {
    Pending,
    Succeeded,
    Failed,
}

impl_writeable_tlv_based_enum!(HTLCStatus,
	(0, Pending) => {},
	(1, Succeeded) => {},
	(2, Failed) => {},
);

pub struct MillisatAmount(pub Option<u64>);

impl fmt::Display for MillisatAmount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Some(amt) => write!(f, "{}", amt),
            None => write!(f, "unknown"),
        }
    }
}

impl Readable for MillisatAmount {
    fn read<R: io::Read>(r: &mut R) -> Result<Self, DecodeError> {
        let amt: Option<u64> = Readable::read(r)?;
        Ok(MillisatAmount(amt))
    }
}

impl Writeable for MillisatAmount {
    fn write<W: Writer>(&self, w: &mut W) -> Result<(), io::Error> {
        self.0.write(w)
    }
}

pub struct PaymentInfo {
    pub preimage: Option<PaymentPreimage>,
    pub secret: Option<PaymentSecret>,
    pub status: HTLCStatus,
    pub amt_msat: MillisatAmount,
}

impl_writeable_tlv_based!(PaymentInfo, {
	(0, preimage, required),
	(2, secret, required),
	(4, status, required),
	(6, amt_msat, required),
});

pub struct InboundPaymentInfoStorage {
    pub payments: HashMap<PaymentHash, PaymentInfo>,
}

impl_writeable_tlv_based!(InboundPaymentInfoStorage, {
	(0, payments, required),
});

pub struct OutboundPaymentInfoStorage {
    pub payments: HashMap<PaymentId, PaymentInfo>,
}

impl_writeable_tlv_based!(OutboundPaymentInfoStorage, {
	(0, payments, required),
});

pub type ChainMonitor = chainmonitor::ChainMonitor<
    InMemorySigner,
    Arc<dyn Filter + Send + Sync>,
    Arc<BitcoindClient>,
    Arc<BitcoindClient>,
    Arc<FilesystemLogger>,
    Arc<
        MonitorUpdatingPersister<
            Arc<FilesystemStore>,
            Arc<FilesystemLogger>,
            Arc<KeysManager>,
            Arc<KeysManager>,
            Arc<BitcoindClient>,
            Arc<BitcoindClient>,
        >,
    >,
>;

pub type GossipVerifier = lightning_block_sync::gossip::GossipVerifier<
    lightning_block_sync::gossip::TokioSpawner,
    Arc<lightning_block_sync::rpc::RpcClient>,
    Arc<FilesystemLogger>,
>;

// Note that if you do not use an `OMDomainResolver` here you should use SimpleArcPeerManager
// instead.
pub type PeerManager = LdkPeerManager<
    SocketDescriptor,
    Arc<ChannelManager>,
    Arc<P2PGossipSync<Arc<NetworkGraph>, Arc<GossipVerifier>, Arc<FilesystemLogger>>>,
    Arc<OnionMessenger>,
    Arc<FilesystemLogger>,
    IgnoringMessageHandler,
    Arc<KeysManager>,
>;

pub type ChannelManager =
SimpleArcChannelManager<ChainMonitor, BitcoindClient, BitcoindClient, FilesystemLogger>;

pub type NetworkGraph = gossip::NetworkGraph<Arc<FilesystemLogger>>;

// Note that if you do not use an `OMDomainResolver` here you should use SimpleArcOnionMessenger
// instead.
pub type OnionMessenger = LdkOnionMessenger<
    Arc<KeysManager>,
    Arc<KeysManager>,
    Arc<FilesystemLogger>,
    Arc<ChannelManager>,
    Arc<DefaultMessageRouter<Arc<NetworkGraph>, Arc<FilesystemLogger>, Arc<KeysManager>>>,
    Arc<ChannelManager>,
    Arc<ChannelManager>,
    Arc<OMDomainResolver<Arc<ChannelManager>>>,
    IgnoringMessageHandler,
>;

pub type BumpTxEventHandler = BumpTransactionEventHandler<
    Arc<BitcoindClient>,
    Arc<Wallet<Arc<BitcoindClient>, Arc<FilesystemLogger>>>,
    Arc<KeysManager>,
    Arc<FilesystemLogger>,
>;

pub type OutputSweeper = ldk_sweep::OutputSweeper<
    Arc<BitcoindClient>,
    Arc<BitcoindClient>,
    Arc<BitcoindClient>,
    Arc<dyn Filter + Send + Sync>,
    Arc<FilesystemStore>,
    Arc<FilesystemLogger>,
    Arc<KeysManager>,
>;

// Needed due to rust-lang/rust#63033.
pub struct OutputSweeperWrapper(pub Arc<OutputSweeper>);
