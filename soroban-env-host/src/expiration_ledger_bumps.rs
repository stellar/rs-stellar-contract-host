use std::rc::Rc;

use crate::{xdr::LedgerKey, MeteredVector};

#[derive(Clone)]
pub struct LedgerBump {
    pub key: Rc<LedgerKey>,
    pub min_expiration: u32,
}

#[derive(Clone, Default)]
pub struct ExpirationLedgerBumps(pub MeteredVector<LedgerBump>);
