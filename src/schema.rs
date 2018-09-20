//! Database schema

use exonum::{
    crypto::{Hash, PublicKey}, storage::{Fork, ProofListIndex, ProofMapIndex, Snapshot},
    blockchain::ExecutionError,
};

use SERVICE_NAME;

encoding_struct! {
    /// Wallet information stored in the database.
    struct Wallet {
        /// `PublicKey` of the wallet.
        pub_key: &PublicKey,
        /// Name of the wallet.
        name:    &str,
        /// Current balance of the wallet.
        balance: u64,
        /// Amount frozen due to active bids
        frozen:  u64,
    }
}

encoding_struct! {
    /// Database entity for a single auction lot
    struct Lot {
        /// `PublicKey` of the lot's creator
        owner: &PublicKey,
        /// Lot name
        name: &str,
        /// Minimum starting bid
        min_bid: u64,
        /// Hash of the transaction that created this lot
        tx_hash: &Hash,
    }
}

encoding_struct! {
    /// A bid
    struct Bid {
        /// `PublicKey` of a wallet owner placing the bid
        owner: &PublicKey,
        /// Bid amount
        amount: u64,
        /// Hash of the transaction that created this bid
        tx_hash: &Hash,
    }
}

use tx::Error;

impl Wallet {
    /// Attempts to freeze a given amount in the wallet's balance
    /// or returns Error::InsufficientCurrencyAmount
    ///
    /// # Arguments
    /// `amount` - the amount to freeze (u64)
    pub fn freeze(self, amount: u64) -> Result<Self, Error> {
        if self.balance() - self.frozen() >= amount {
            Ok(Self::new(self.pub_key(), self.name(), self.balance() - amount, self.frozen() + amount))
        } else {
            Err(Error::InsufficientCurrencyAmount)?
        }
    }

    /// Releases a given amount in the wallet's balance
    /// If the requested amount is greater than the currently frozen one, everything is released
    ///
    /// # Arguments
    /// `amount` - the amount to release (u64)
    pub fn release(self, amount: u64) -> Self {
        let actual_amount = if self.frozen() <= amount { self.frozen() } else { amount };
        Self::new(self.pub_key(), self.name(), self.balance() + actual_amount, self.frozen() - actual_amount)
    }
}

/// DB schema
#[derive(Debug)]
pub struct Schema<T> {
    view: T,
}

impl<T> AsMut<T> for Schema<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.view
    }
}

impl<T> Schema<T>
    where
        T: AsRef<dyn Snapshot>,
{
    /// Creates a new schema from the database view.
    pub fn new(view: T) -> Self {
        Schema { view }
    }

    /// Returns `ProofMapIndex` with wallets.
    pub fn wallets(&self) -> ProofMapIndex<&T, PublicKey, Wallet> {
        ProofMapIndex::new(format!("{}.wallets", SERVICE_NAME), &self.view)
    }

    /// Returns a `ProofMapIndex` with lots.
    pub fn lots(&self) -> ProofMapIndex<&T, Hash, Lot> {
        ProofMapIndex::new(format!("{}.lots", SERVICE_NAME), &self.view)
    }

    /// Returns bid history for a lot with the given hash.
    pub fn bid_history(&self, hash: &Hash) -> ProofListIndex<&T, Bid> {
        ProofListIndex::new_in_family(format!("{}.bid_history", SERVICE_NAME), hash, &self.view)
    }

    /// Returns the wallet for the given public key.
    pub fn wallet(&self, pub_key: &PublicKey) -> Option<Wallet> {
        self.wallets().get(pub_key)
    }

    /// Returns a lot by its hash.
    pub fn lot(&self, id: &Hash) -> Option<Lot> {
        self.lots().get(id)
    }

    /// Returns the last bid in a lot's bid history
    pub fn last_bid(&self, id: &Hash) -> Option<Bid> {
        match self.lot(id) {
            Some(_lot) => self.bid_history(id).last(),
            None => None,
        }
    }

    /// Returns the service state hash
    pub fn state_hash(&self) -> Vec<Hash> {
        vec![self.wallets().merkle_root()]
    }
}

impl<'a> Schema<&'a mut Fork> {
    /// Returns mutable `ProofMapIndex` with wallets.
    pub fn wallets_mut(&mut self) -> ProofMapIndex<&mut Fork, PublicKey, Wallet> {
        ProofMapIndex::new(format!("{}.wallets", SERVICE_NAME), &mut self.view)
    }

    /// Mutable version of the `lots` method
    pub fn lots_mut(&mut self) -> ProofMapIndex<&mut Fork, Hash, Lot> {
        ProofMapIndex::new(format!("{}.lots", SERVICE_NAME), &mut self.view)
    }

    /// Mutable version of the `bid_history` method
    pub fn bid_history_mut(&mut self, lot: &Hash) -> ProofListIndex<&mut Fork, Bid> {
        ProofListIndex::new_in_family(format!("{}.bid_history", SERVICE_NAME), lot, &mut self.view)
    }

    /// Creates a new wallet
    pub fn create_wallet(&mut self, key: &PublicKey, name: &str, balance: u64) {
        let wallet = Wallet::new(key, name, balance, 0);
        self.wallets_mut().put(key, wallet);
    }

    /// Creates a new lot
    ///
    /// # Arguments
    /// - `owner`: lot creator's public key
    /// - `name`: name of the lot
    /// - `min_bid`: starting bid amount
    pub fn create_lot(&mut self, owner: &PublicKey, name: &str, min_bid: u64, hash: &Hash) {
        let lot = Lot::new(owner, name, min_bid, hash);
        self.lots_mut().put(hash, lot);
    }

    /// Attempts to place a new bid on a given lot
    ///
    /// # Arguments
    /// - `owner`: lot creator's public key
    /// - `name`: name of the lot
    /// - `min_bid`: starting bid amount
    pub fn place_bid(&mut self, owner: &PublicKey, lot: &Hash, amount: u64) -> Result<(), ExecutionError> {
        match self.last_bid(lot) {
            Some(bid) => {
                match self.wallet(bid.owner()) {
                    Some(wallet) => {
                        if amount <= bid.amount() {
                            Err(Error::BidTooLow)?
                        }

                        self.wallets_mut().put(bid.owner(), wallet.release(bid.amount()));
                    },
                    None => {},
                };
            },
            None => {},
        };

        let wallet = match self.wallet(owner) {
            Some(val) => val.freeze(amount)?,
            None => Err(Error::InsufficientCurrencyAmount)?,
        };

        let bid = Bid::new(owner, amount, lot);
        self.bid_history_mut(lot).push(bid);
        self.wallets_mut().put(owner, wallet);
        Ok(())
    }
}
