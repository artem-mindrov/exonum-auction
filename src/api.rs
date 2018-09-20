//! Public API exposed by the auction service
extern crate pub_sub;

use exonum::{
    api::{self, ServiceApiBuilder, ServiceApiState}, blockchain::{Transaction, Schema},
    crypto::{Hash, PublicKey}, node::TransactionSend, helpers::Height,
};

use tx::AuctionTransactions;
use schema::{Bid, Wallet};
use Schema as AuctionSchema;

static mut BLOCK_PS: Option<pub_sub::PubSub<Height>> = None;

/// Describes the query parameters for the `get_wallet` endpoint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct WalletQuery {
    /// Public key of the queried wallet.
    pub pub_key: PublicKey,
}

/// Describes the query parameters for the `bid_history` endpoint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct BidHistoryQuery {
    /// Hash describing the lot to be queried.
    pub id: Hash,
}

/// Asynchronous response to an incoming transaction returned by the REST API.
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionResponse {
    /// Hash of the transaction.
    pub tx_hash: Hash,
}

/// Response returned by the `POST /wallets` endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWalletResponse {
    /// Hash of the transaction.
    pub tx_hash: Hash,
    /// Public key
    pub pub_key: PublicKey,
}

/// Response to a synchronized transaction request returned by the REST API.
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionSyncResponse {
    /// Hash of the transaction.
    pub tx_hash: Hash,
    /// Height of the block this transaction belongs to
    pub tx_block_height: Height,
}

/// Bid history information.
#[derive(Debug, Serialize, Deserialize)]
pub struct BidHistory {
    /// List of outstanding bids.
    pub bids: Vec<Bid>,
}

/// Public service API description.
#[derive(Debug, Clone, Copy)]
pub struct PublicApi;

impl PublicApi {
    /// Endpoint for getting a single wallet.
    pub fn wallet(state: &ServiceApiState, query: WalletQuery) -> api::Result<Wallet> {
        let snapshot = state.snapshot();
        let schema = AuctionSchema::new(&snapshot);
        schema.wallet(&query.pub_key).ok_or_else(|| api::Error::NotFound("\"Wallet not found\"".to_owned()))
    }

    /// Endpoint for retrieving full bid history for a single lot
    pub fn bid_history(state: &ServiceApiState, query: BidHistoryQuery) -> api::Result<BidHistory> {
        let snapshot = state.snapshot();
        let schema = AuctionSchema::new(&snapshot);
        let history = schema.bid_history(&query.id);
        let bids = history.iter().collect::<Vec<_>>();
        Ok(BidHistory { bids })
    }

    /// Endpoint for handling asynchronous transactions.
    pub fn post_transaction(
        state: &ServiceApiState,
        query: AuctionTransactions,
    ) -> api::Result<TransactionResponse> {
        let transaction: Box<dyn Transaction> = query.into();
        let tx_hash = transaction.hash();
        state.sender().send(transaction)?;
        Ok(TransactionResponse { tx_hash })
    }

    /// This is a blocking request that will wait till the block with the associated transaction
    /// is committed
    pub fn post_transaction_sync(
        state: &ServiceApiState,
        query: AuctionTransactions,
    ) -> api::Result<TransactionSyncResponse> {
        let transaction: Box<dyn Transaction> = query.into();
        let tx_hash = transaction.hash();
        state.sender().send(transaction)?;

        unsafe {
            let rx = &BLOCK_PS.as_ref().unwrap();
            let recv = rx.subscribe();

            loop { // TODO: decide on a reasonable timeout, should probably be configurable
                let tx_block_height = recv.recv().unwrap();
                let snapshot = state.snapshot();
                let schema = Schema::new(&snapshot);
                let txs = schema.block_transactions(tx_block_height);
                for tx in txs.iter() {
                    if tx == tx_hash {
                        return Ok(TransactionSyncResponse { tx_hash, tx_block_height });
                    }
                }
            }
        }
    }

    /// Called by the after_commit handler to send the last block height back
    /// to the requests currently blocked on the commit result
    pub unsafe fn sync_commit_callback(height: Height) {
        match BLOCK_PS.as_ref() {
            Some(tx) => tx.clone().send(height).unwrap(),
            None => {},
        }
    }

    /// Wires the above endpoint to public scope of the given `ServiceApiBuilder`.
    pub fn wire(builder: &mut ServiceApiBuilder) {
        unsafe {
            BLOCK_PS = Some(pub_sub::PubSub::new());
        }

        builder
            .public_scope()
            .endpoint("v1/wallet", Self::wallet)
            .endpoint("v1/bids", Self::bid_history)
            .endpoint_mut("v1/bids", Self::post_transaction_sync)
            .endpoint_mut("v1/lots", Self::post_transaction)
            .endpoint_mut("v1/wallets", Self::post_transaction);
    }
}
