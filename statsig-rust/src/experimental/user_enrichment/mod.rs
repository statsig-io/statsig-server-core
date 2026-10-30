//! **Experimental.** Forward-proxy user enrichment: resolve a user's ID-list
//! memberships via `POST /v1/user_enrichment` on a Statsig forward proxy and
//! attach them to a `StatsigUser` so the evaluator can answer ID-list
//! conditions without holding the lists in memory.
//!
//! This API is unstable and may change without notice. See
//! [`crate::experimental`].

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::time::Duration;

pub mod forward_proxy;
pub mod fp_user_builder;

pub use forward_proxy::{ForwardProxyEnrichmentOptions, ForwardProxyUserEnrichmentProvider};
pub use fp_user_builder::StatsigFPUserBuilder;

/// Hashed unit identifiers sent to an enrichment provider.
pub type IdListLookups = Vec<String>;

/// Lookup ID to every ID-list name containing that lookup.
pub type IdListMemberships = HashMap<String, HashSet<String>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnrichmentError {
    Timeout(Duration),
    Http(u16),
    Transport(String),
    Decode(String),
}

impl Display for EnrichmentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout(duration) => {
                write!(formatter, "enrichment request timed out after {duration:?}")
            }
            Self::Http(status) => write!(formatter, "enrichment request returned HTTP {status}"),
            Self::Transport(message) => write!(formatter, "enrichment transport failed: {message}"),
            Self::Decode(message) => {
                write!(formatter, "enrichment response decode failed: {message}")
            }
        }
    }
}

impl std::error::Error for EnrichmentError {}

#[allow(
    clippy::double_must_use,
    reason = "async_trait adds must_use to methods returning an already-must-use boxed Future"
)]
#[async_trait]
pub trait IDListEnrichmentProvider: Send + Sync {
    /// Resolves hashed lookup IDs. Implementations must return an error rather
    /// than fabricating an empty membership map when transport or decoding
    /// fails.
    async fn resolve_id_list_memberships(
        &self,
        lookups: &IdListLookups,
    ) -> Result<IdListMemberships, EnrichmentError>;

    fn provider_name(&self) -> &'static str {
        "custom"
    }

    fn endpoint(&self) -> &str {
        ""
    }
}
