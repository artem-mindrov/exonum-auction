//! Integration tests

extern crate assert_matches;
extern crate exonum;
extern crate exonum_auction as auction;
//extern crate exonum_crypto;
extern crate exonum_testkit;
#[macro_use]
extern crate serde_json;

use exonum::{
    api::{node::public::explorer::TransactionQuery},
    crypto::{self, CryptoHash, Hash, hash, PublicKey, SecretKey},
};
use exonum_testkit::{ApiKind, TestKit, TestKitApi, TestKitBuilder};
use std::{thread, time};
//use std::sync::mpsc::channel;
mod common;

const BLOCK_DELAY_SEC: u64 = 1;

// Import data types used in tests from the crate where the service is defined.
use auction::api::{WalletQuery, BidHistoryQuery, BidHistory};
use auction::schema::Wallet;
use auction::tx::{CreateWallet, CreateLot, PlaceBid};
use common::{PHOBOS, DEIMOS, MIN_BID, BALANCE};

/// Check that the wallet creation transaction works when invoked via API.
#[test]
fn test_api_create_wallet() {
    let (mut testkit, api) = create_testkit();
    let (tx, _) = api.create_wallet(PHOBOS);
    testkit.create_block();
    api.assert_tx_status(tx.hash(), &json!({ "type": "success" }));

    let wallet = api.wallet(*tx.pub_key());
    assert_eq!(wallet.pub_key(), tx.pub_key());
    assert_eq!(wallet.name(), tx.name());
    assert_eq!(wallet.balance(), BALANCE);
}

/// Test lot creation
#[test]
fn test_api_create_lot() {
    let (mut testkit, api) = create_testkit();
    let (tx, key) = api.create_wallet(PHOBOS);
    testkit.create_block();
    api.assert_tx_status(tx.hash(), &json!({ "type": "success" }));
    let ltx = api.create_lot(&tx.pub_key(), &key);
    testkit.create_block();
    api.assert_tx_status(ltx.hash(), &json!({ "type": "success" }));
}

#[test]
fn test_api_place_bid() {
    let (mut testkit, api) = create_testkit();
    let (tx, key) = api.create_wallet(PHOBOS);
    let ltx = api.create_lot(&tx.pub_key(), &key);
    let (bidder_tx, key) = api.create_wallet(DEIMOS);
    testkit.create_block_with_tx_hashes(&[tx.hash(), ltx.hash(), bidder_tx.hash()]);

    thread::spawn(move || {
        thread::sleep(time::Duration::from_secs(BLOCK_DELAY_SEC));
        testkit.create_block();
    });
    let btx= api.place_bid(&bidder_tx.pub_key(), &ltx.hash(), MIN_BID, &key, 2);
//    testkit.create_block_with_tx_hashes(&[tx.hash(), ltx.hash(), bidder_tx.hash(), btx.hash()]);
    api.assert_tx_status(btx.hash(), &json!({ "type": "success" }));

    let bid_history = api.bid_history(ltx.hash());
    assert_eq!(bid_history.bids.len(), 1);

    let last_bid = bid_history.bids.last().unwrap();
    assert_eq!(last_bid.owner(), bidder_tx.pub_key());
    assert_eq!(last_bid.amount(), MIN_BID);
}

#[test]
fn test_api_create_existing_wallet() {
    let (mut testkit, api) = create_testkit();
    let (tx, key) = api.create_wallet(PHOBOS);
    testkit.create_block();

    let dup_tx = CreateWallet::new(&tx.pub_key(), "duplicate wallet", BALANCE, &key);
    let _tx_info: serde_json::Value = api.inner
        .public(ApiKind::Service(auction::SERVICE_NAME))
        .query(&dup_tx)
        .post("v1/wallets")
        .unwrap();
    testkit.create_block_with_tx_hashes(&[dup_tx.hash()]);
    api.assert_tx_status(
        dup_tx.hash(),
        &json!({ "type": "error", "code": 0, "description": "Wallet already exists" }),
    );
}

#[test]
fn test_api_create_lot_for_nonexistent_wallet() {
    let (mut testkit, api) = create_testkit();
    let (pubkey, key) = crypto::gen_keypair();
    let ltx = api.create_lot(&pubkey, &key);
    testkit.create_block();
    api.assert_tx_status(
        ltx.hash(),
        &json!({ "type": "error", "code": 4, "description": "Wallet does not exist" }),
    );
}

#[test]
fn test_api_place_bid_on_nonexistent_lot() {
    let (mut testkit, api) = create_testkit();
    let (tx, key) = api.create_wallet(PHOBOS);
    testkit.create_block();
    let data = [1, 2, 3];
    let hash = hash(&data);

    thread::spawn(move || {
        thread::sleep(time::Duration::from_secs(BLOCK_DELAY_SEC));
        testkit.create_block();
    });
    let btx= api.place_bid(&tx.pub_key(), &hash, MIN_BID, &key, 2);

    api.assert_tx_status(
        btx.hash(),
        &json!({ "type": "error", "code": 1, "description": "Lot does not exist" }),
    );
}

#[test]
fn test_api_place_bid_on_own_lot() {
    let (mut testkit, api) = create_testkit();
    let (tx, key) = api.create_wallet(PHOBOS);
    let ltx = api.create_lot(&tx.pub_key(), &key);
    testkit.create_block_with_tx_hashes(&[tx.hash(), ltx.hash()]);

    thread::spawn(move || {
        thread::sleep(time::Duration::from_secs(BLOCK_DELAY_SEC));
        testkit.create_block();
    });
    let btx= api.place_bid(&tx.pub_key(), &ltx.hash(), MIN_BID, &key, 2);

    api.assert_tx_status(
        btx.hash(),
        &json!({ "type": "error", "code": 5, "description": "Bidding not allowed on one's own lot" }),
    );
}

#[test]
fn test_api_place_bid_below_minimum() {
    let (mut testkit, api) = create_testkit();
    let (tx, key) = api.create_wallet(PHOBOS);
    let ltx = api.create_lot(&tx.pub_key(), &key);
    let (bidder_tx, key) = api.create_wallet(DEIMOS);
    testkit.create_block_with_tx_hashes(&[tx.hash(), ltx.hash(), bidder_tx.hash()]);

    thread::spawn(move || {
        thread::sleep(time::Duration::from_secs(BLOCK_DELAY_SEC));
        testkit.create_block();
    });
    let btx= api.place_bid(&bidder_tx.pub_key(), &ltx.hash(), MIN_BID - 1, &key, 2);

    api.assert_tx_status(
        btx.hash(),
        &json!({ "type": "error", "code": 2, "description": "Bid below current minimum" }),
    );
}

#[test]
fn test_api_place_bid_above_balance() {
    let (mut testkit, api) = create_testkit();
    let (tx, key) = api.create_wallet(PHOBOS);
    let ltx = api.create_lot(&tx.pub_key(), &key);
    let (bidder_tx, key) = api.create_wallet(DEIMOS);
    testkit.create_block_with_tx_hashes(&[tx.hash(), ltx.hash(), bidder_tx.hash()]);

    thread::spawn(move || {
        thread::sleep(time::Duration::from_secs(BLOCK_DELAY_SEC));
        testkit.create_block();
    });
    let btx = api.place_bid(&bidder_tx.pub_key(), &ltx.hash(), BALANCE + 1, &key, 2);

    api.assert_tx_status(
        btx.hash(),
        &json!({ "type": "error", "code": 3, "description": "Currency amount insufficient for bid placement" }),
    );
}

struct ApiWrapper {
    pub inner: TestKitApi,
}

impl ApiWrapper {
    fn create_wallet(&self, name: &str) -> (CreateWallet, SecretKey) {
        let (pubkey, key) = crypto::gen_keypair();
        // Create a pre-signed transaction
        let tx = CreateWallet::new(&pubkey, name, BALANCE, &key);

        let tx_info: serde_json::Value = self.inner
            .public(ApiKind::Service(auction::SERVICE_NAME))
            .query(&tx)
            .post("v1/wallets")
            .unwrap();
        assert_eq!(tx_info, json!({ "tx_hash": tx.hash() }));
        (tx, key)
    }

    /// Gets the state of a particular wallet using an HTTP request.
    fn wallet(&self, pub_key: PublicKey) -> Wallet {
        self.inner
            .public(ApiKind::Service(auction::SERVICE_NAME))
            .query(&WalletQuery { pub_key })
            .get("v1/wallet")
            .unwrap()
    }

    fn bid_history(&self, lot_id: Hash) -> BidHistory {
        self.inner
            .public(ApiKind::Service(auction::SERVICE_NAME))
            .query(&BidHistoryQuery { id: lot_id })
            .get("v1/bids")
            .unwrap()
    }

    /// Asserts that the transaction with the given hash has a specified status.
    fn assert_tx_status(&self, tx_hash: Hash, expected_status: &serde_json::Value) {
        let info: serde_json::Value = self.inner
            .public(ApiKind::Explorer)
            .query(&TransactionQuery::new(tx_hash))
            .get("v1/transactions")
            .unwrap();

        if let serde_json::Value::Object(mut info) = info {
            let tx_status = info.remove("status").unwrap();
            assert_eq!(tx_status, *expected_status);
        } else {
            panic!("Invalid transaction info format, object expected");
        }
    }

    /// Creates a lot given a participant's public key
    fn create_lot(&self, owner: &PublicKey, key: &SecretKey) -> CreateLot {
        let ltx = CreateLot::new(owner, "Test lot", MIN_BID, key);

        let tx_info: serde_json::Value = self.inner
            .public(ApiKind::Service(auction::SERVICE_NAME))
            .query(&ltx)
            .post("v1/lots")
            .unwrap();
        assert_eq!(tx_info, json!({ "tx_hash": ltx.hash() }));
        ltx
    }

    fn place_bid(&self, bidder: &PublicKey, lot_id: &Hash, bid: u64, key: &SecretKey, expected_height: u64) -> PlaceBid {
        let btx = PlaceBid::new(bidder, lot_id, bid, key);
        let tx_info: serde_json::Value = self.inner
            .public(ApiKind::Service(auction::SERVICE_NAME))
            .query(&btx)
            .post("v1/bids")
            .unwrap();
        assert_eq!(tx_info, json!({ "tx_hash": btx.hash(), "tx_block_height": expected_height }));
        btx
    }
}

/// Creates a testkit together with the API wrapper defined above.
fn create_testkit() -> (TestKit, ApiWrapper) {
    let testkit = TestKitBuilder::validator()
        .with_service(auction::Service)
        .create();
    let api = ApiWrapper {
        inner: testkit.api(),
    };
    (testkit, api)
}
