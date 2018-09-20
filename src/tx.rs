//! Transaction definition and implementation

#![allow(bare_trait_objects)]

use exonum::{
    blockchain::{ExecutionError, ExecutionResult, Transaction}, crypto::{CryptoHash, Hash, PublicKey},
    messages::Message, storage::Fork,
};

use schema::Schema;
use SERVICE_ID;

/// Error codes returned by the service transactions
#[derive(Debug, Fail)]
#[repr(u8)]
pub enum Error {
    /// Wallet already exists.
    ///
    /// Can be emitted by `CreateWallet`.
    #[fail(display = "Wallet already exists")]
    WalletAlreadyExists = 0,

    /// Lot doesn't exist.
    ///
    /// Can be emitted by `PlaceBid`.
    #[fail(display = "Lot does not exist")]
    LotNotFound = 1,

    /// Bid too low.
    ///
    /// Can be emitted by `PlaceBid`.
    #[fail(display = "Bid below current minimum")]
    BidTooLow = 2,

    /// Insufficient currency amount.
    ///
    /// Can be emitted by `PlaceBid`.
    #[fail(display = "Currency amount insufficient for bid placement")]
    InsufficientCurrencyAmount = 3,

    /// Wallet does not exist.
    ///
    /// Can be emitted by `PlaceBid` and `CreateLot`.
    #[fail(display = "Wallet does not exist")]
    WalletNotFound = 4,

    /// Participants can't bid for their own lots.
    ///
    /// Can be emitted by `PlaceBid`.
    #[fail(display = "Bidding not allowed on one's own lot")]
    BiddingNotAllowedOnOwnLot = 5,
}

impl From<Error> for ExecutionError {
    fn from(value: Error) -> ExecutionError {
        let description = format!("{}", value);
        ExecutionError::with_description(value as u8, description)
    }
}

transactions! {
    /// Transaction group.
    pub AuctionTransactions {
        const SERVICE_ID = SERVICE_ID;

        /// Create a wallet with the given `name`.
        struct CreateWallet {
            /// `PublicKey` of the new wallet.
            pub_key: &PublicKey,
            /// Name of the new wallet.
            name:    &str,
            /// Initial balance
            balance: u64,
        }

        /// Create a lot with the given name and starting bid amount
        struct CreateLot {
            /// Lot owner
            owner: &PublicKey,
            /// Lot name
            name:  &str,
            /// Minimum bid
            min_bid: u64,
        }

        /// Bid placement
        struct PlaceBid {
            /// Bid initiator
            owner: &PublicKey,
            /// ID (hash) of the lot to bid on
            lot: &Hash,
            /// Bid amount
            amount: u64,
        }
    }
}

impl Transaction for CreateWallet {
    fn verify(&self) -> bool {
        self.verify_signature(self.pub_key())
    }

    fn execute(&self, fork: &mut Fork) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let pub_key = self.pub_key();

        if schema.wallet(pub_key).is_none() {
            schema.create_wallet(pub_key, self.name(), self.balance());
            Ok(())
        } else {
            Err(Error::WalletAlreadyExists)?
        }
    }
}

impl Transaction for CreateLot {
    fn verify(&self) -> bool {
        self.verify_signature(self.owner())
    }

    fn execute(&self, fork: &mut Fork) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let owner = self.owner();

        if schema.wallet(owner).is_none() {
            Err(Error::WalletNotFound)?
        } else {
            schema.create_lot(owner, self.name(), self.min_bid(), &self.hash());
            Ok(())
        }
    }
}

impl Transaction for PlaceBid {
    fn verify(&self) -> bool {
        self.verify_signature(self.owner())
    }

    fn execute(&self, fork: &mut Fork) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let owner = self.owner();
        let lot = match schema.lot(self.lot()) {
            Some(val) => val,
            None => Err(Error::LotNotFound)?,
        };

        if lot.min_bid() > self.amount() {
            Err(Error::BidTooLow)?
        }

        if lot.owner() == owner {
            Err(Error::BiddingNotAllowedOnOwnLot)?
        } else {
            schema.place_bid(owner, lot.tx_hash(), self.amount())
        }
    }
}
