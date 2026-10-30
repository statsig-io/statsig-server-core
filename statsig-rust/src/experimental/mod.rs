//! Experimental APIs.
//!
//! Everything under `statsig_rust::experimental` is unstable. It may change
//! shape or be removed in any release without a major version bump, and it
//! is not covered by the SDK's compatibility guarantees. The `experimental`
//! segment is deliberately part of every import path so that call sites make
//! the dependency explicit:
//!
//! ```ignore
//! use statsig_rust::experimental::{
//!     ForwardProxyEnrichmentOptions, ForwardProxyUserEnrichmentProvider, StatsigFPUserBuilder,
//! };
//! ```

pub mod user_enrichment;

pub use user_enrichment::{
    EnrichmentError, ForwardProxyEnrichmentOptions, ForwardProxyUserEnrichmentProvider,
    IDListEnrichmentProvider, IdListLookups, IdListMemberships, StatsigFPUserBuilder,
};
