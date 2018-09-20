//! A simple auction service implementation example using [exonum](http://exonum.com/).
#![deny(missing_debug_implementations, missing_docs, bare_trait_objects)]

#[macro_use]
extern crate exonum;
#[macro_use]
extern crate failure;
extern crate serde;
#[macro_use]
extern crate serde_derive;

pub use schema::Schema;

pub mod api;
pub mod schema;
pub mod tx;

use exonum::{
    api::ServiceApiBuilder, blockchain::{self, Transaction, TransactionSet, ServiceContext}, crypto::Hash,
    encoding::Error as EncodingError, helpers::fabric::{self, Context}, messages::RawTransaction,
    storage::Snapshot,
};

use tx::AuctionTransactions;

const SERVICE_ID: u16 = 42;
/// Name of the service.
pub const SERVICE_NAME: &str = "auction";

/// Service implementation
#[derive(Default, Debug)]
pub struct Service;

impl blockchain::Service for Service {
    fn service_id(&self) -> u16 {
        SERVICE_ID
    }

    fn service_name(&self) -> &str {
        SERVICE_NAME
    }

    fn state_hash(&self, view: &dyn Snapshot) -> Vec<Hash> {
        let schema = Schema::new(view);
        schema.state_hash()
    }

    fn tx_from_raw(&self, raw: RawTransaction) -> Result<Box<dyn Transaction>, EncodingError> {
        AuctionTransactions::tx_from_raw(raw).map(Into::into)
    }

    fn after_commit(&self, context: &ServiceContext) {
        unsafe { api::PublicApi::sync_commit_callback(context.height()); }
    }

    fn wire_api(&self, builder: &mut ServiceApiBuilder) {
        api::PublicApi::wire(builder);
    }
}

/// A configuration service creator for the `NodeBuilder`.
#[derive(Debug)]
pub struct ServiceFactory;

impl fabric::ServiceFactory for ServiceFactory {
    fn service_name(&self) -> &str {
        SERVICE_NAME
    }

    fn make_service(&mut self, _: &Context) -> Box<dyn blockchain::Service> {
        Box::new(Service)
    }
}
