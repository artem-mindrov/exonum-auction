//! Unit tests
extern crate exonum;
//extern crate exonum_crypto;
extern crate exonum_auction as auction;
extern crate exonum_testkit;
extern crate rand;

use exonum::{
    crypto::{self, PublicKey, SecretKey, Hash, hash, CryptoHash},
};
use exonum_testkit::{TestKit, TestKitBuilder};

// Import data types used in tests from the crate where the service is defined.
use auction::{
    schema::{Schema, Wallet, Lot, Bid}, tx::{CreateWallet, CreateLot, PlaceBid},
};

mod common;
use common::{PHOBOS, DEIMOS, MIN_BID, BALANCE};

#[test]
fn test_tx_create_wallet() {
    let mut testkit = init_testkit();
    let (tx, _) = create_wallet(&mut testkit, PHOBOS, BALANCE);

    // Check that the user indeed is persisted by the service
    let wallet = get_wallet(&testkit, tx.pub_key());
    assert_eq!(wallet.pub_key(), tx.pub_key());
    assert_eq!(wallet.name(), PHOBOS);
    assert_eq!(wallet.balance(), BALANCE);
}

#[test]
fn test_tx_create_lot() {
    let mut testkit = init_testkit();
    let lot_name = format!("{}'s test lot", PHOBOS);
    let (ltx, wtx, _) = create_lot(&mut testkit, lot_name.as_str(), MIN_BID);

    // Check that the user indeed is persisted by the service
    let lot = get_lot(&testkit, &ltx.hash());
    assert_eq!(lot.owner(), wtx.pub_key());
    assert_eq!(lot.name(), lot_name.as_str());
    assert_eq!(lot.min_bid(), MIN_BID);
}

#[test]
fn test_tx_place_bids() {
    let mut testkit = init_testkit();
    let (ltx, _, _) = create_lot(&mut testkit, format!("{}'s test lot", PHOBOS).as_str(), MIN_BID);

    let (tx_bidder, key) = create_wallet(&mut testkit, DEIMOS, BALANCE);

    for n in 1..=2 {
        let amount = MIN_BID + n - 1;
        let _btx = place_bid(&mut testkit, &tx_bidder.pub_key(), &key,&ltx.hash(), amount);
        let bid = last_bid(&testkit, &ltx.hash());
        assert_eq!(bid_history_size(&testkit, &ltx.hash()), n);
        assert_eq!(bid.owner(), tx_bidder.pub_key());
        assert_eq!(bid.amount(), amount);

        let bidder_wallet = get_wallet(&testkit, tx_bidder.pub_key());
        assert_eq!(bidder_wallet.balance(), BALANCE - amount);
    }
}

#[test]
fn test_tx_create_existing_wallet() {
    let mut testkit = init_testkit();
    let (tx, key) = create_wallet(&mut testkit, PHOBOS, BALANCE);
    testkit.create_block_with_transaction(CreateWallet::new(&tx.pub_key(), format!("{}'s test lot", PHOBOS).as_str(), BALANCE + 20, &key));

    // Check that the user indeed is persisted by the service
    let wallet = get_wallet(&testkit, tx.pub_key());
    assert_eq!(wallet.name(), PHOBOS);
    assert_eq!(wallet.balance(), BALANCE);
}

#[test]
fn test_tx_create_lot_for_nonexistent_wallet() {
    let mut testkit = init_testkit();
    let (pubkey, key) = crypto::gen_keypair();
    testkit.create_block_with_transaction(CreateLot::new(&pubkey, "test", 0, &key));

    assert_eq!(lots_total(&testkit), 0);
}

#[test]
fn test_tx_place_bid_on_nonexistent_lot() {
    let mut testkit = init_testkit();
    let (tx, key) = create_wallet(&mut testkit, PHOBOS, BALANCE);
    let data = [1, 2, 3];
    let hash = hash(&data);
    testkit.create_block_with_transaction(PlaceBid::new(&tx.pub_key(), &hash, 10, &key));

    assert_eq!(bid_history_size(&testkit, &hash), 0);
}

#[test]
fn test_tx_place_bid_on_own_lot() {
    let mut testkit = init_testkit();
    let (ltx, wtx, key) = create_lot(&mut testkit, format!("{}'s test lot", PHOBOS).as_str(), MIN_BID);

    let _btx = place_bid(&mut testkit, &wtx.pub_key(), &key, &ltx.hash(), MIN_BID);
    assert_eq!(bid_history_size(&testkit, &ltx.hash()), 0);
}

#[test]
fn test_tx_place_bid_below_minimum() {
    let mut testkit = init_testkit();
    let (ltx, _wtx, _) = create_lot(&mut testkit, format!("{}'s test lot", PHOBOS).as_str(), MIN_BID);
    let (tx_bidder, key) = create_wallet(&mut testkit, DEIMOS, BALANCE);

    let _btx = place_bid(&mut testkit, &tx_bidder.pub_key(), &key, &ltx.hash(), MIN_BID - 1);
    assert_eq!(bid_history_size(&testkit, &ltx.hash()), 0);
}

#[test]
fn test_tx_place_bid_below_current_highest_bid() {
    let mut testkit = init_testkit();
    let (ltx, _wtx, _) = create_lot(&mut testkit, format!("{}'s test lot", PHOBOS).as_str(), MIN_BID);
    let (tx_bidder, key) = create_wallet(&mut testkit, DEIMOS, BALANCE);

    for n in 1..=2 {
        let _btx = place_bid(&mut testkit, &tx_bidder.pub_key(), &key,&ltx.hash(), MIN_BID + n - 1);
    }

    let _btx = place_bid(&mut testkit, &tx_bidder.pub_key(), &key, &ltx.hash(), MIN_BID - 1);
    assert_eq!(bid_history_size(&testkit, &ltx.hash()), 2);
}

#[test]
fn test_tx_place_bid_above_balance() {
    let mut testkit = init_testkit();
    let (ltx, _, _) = create_lot(&mut testkit, format!("{}'s test lot", PHOBOS).as_str(), MIN_BID);
    let (tx_bidder, key) = create_wallet(&mut testkit, DEIMOS, BALANCE);

    let _btx = place_bid(&mut testkit, &tx_bidder.pub_key(), &key, &ltx.hash(), BALANCE + 1);
    assert_eq!(bid_history_size(&testkit, &ltx.hash()), 0);

    let bidder_wallet = get_wallet(&testkit, tx_bidder.pub_key());
    assert_eq!(bidder_wallet.balance(), BALANCE);
}

/// Initializes testkit with `Service`.
fn init_testkit() -> TestKit {
    TestKitBuilder::validator()
        .with_service(auction::Service)
        .create()
}

/// Creates a wallet with the given name and a random key.
fn create_wallet(testkit: &mut TestKit, name: &str, balance: u64) -> (CreateWallet, SecretKey) {
    let (pubkey, key) = crypto::gen_keypair();
    let tx = CreateWallet::new(&pubkey, name, balance, &key);
    testkit.create_block_with_transaction(tx.clone());
    (tx, key)
}

/// Returns the wallet identified by the given public key.
fn get_wallet(testkit: &TestKit, pubkey: &PublicKey) -> Wallet {
    Schema::new(&testkit.snapshot()).wallet(pubkey).expect("No wallet persisted")
}

fn create_lot(testkit: &mut TestKit, name: &str, min_bid: u64) -> (CreateLot, CreateWallet, SecretKey) {
    let (tx, key) = create_wallet(testkit, PHOBOS, BALANCE);

    let ltx = CreateLot::new(&tx.pub_key(), name, min_bid, &key);
    testkit.create_block_with_transaction(ltx.clone());
    (ltx, tx, key)
}

fn get_lot(testkit: &TestKit, id: &Hash) -> Lot {
    Schema::new(&testkit.snapshot()).lot(id).expect("No lot persisted")
}

fn place_bid(testkit: &mut TestKit, bidder: &PublicKey, key: &SecretKey, lot_id: &Hash, bid: u64) -> PlaceBid {
    let tx = PlaceBid::new(&bidder, lot_id, bid, key);
    testkit.create_block_with_transaction(tx.clone());
    tx
}

fn last_bid(testkit: &TestKit, lot_id: &Hash) -> Bid {
    Schema::new(&testkit.snapshot()).last_bid(lot_id).expect("No bid history for lot")
}

fn bid_history_size(testkit: &TestKit, lot_id: &Hash) -> u64 {
    Schema::new(&testkit.snapshot()).bid_history(lot_id).len()
}

fn lots_total(testkit: &TestKit) -> usize {
    let s = testkit.snapshot();
    let schema = Schema::new(&s);
    let lots = schema.lots();
    let c = lots.iter().count();
    c
}
