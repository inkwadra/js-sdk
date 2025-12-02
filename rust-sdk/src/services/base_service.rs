//! Base Service

use crate::Client;
use std::sync::Arc;

/// Base trait for all API services.
pub trait BaseService {
    /// Returns a reference to the client.
    fn client(&self) -> &Arc<Client>;
}
