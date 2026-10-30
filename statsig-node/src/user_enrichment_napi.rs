//! Node bindings for the experimental forward-proxy user enrichment API
//! (`statsig_rust::experimental`). Native exports and generated TypeScript
//! declarations live in the `experimental` namespace.

use crate::statsig_options_napi::ProxyConfig;
use crate::statsig_user_napi::{StatsigUser, StatsigUserArgs};
use napi::bindgen_prelude::Result;
use napi::Error;
use napi_derive::napi;
use statsig_rust::experimental::{
    ForwardProxyEnrichmentOptions, ForwardProxyUserEnrichmentProvider, StatsigFPUserBuilder,
};
use statsig_rust::user::user_data::UserData;
use std::sync::Arc;

/// **Experimental.** Options for `ForwardProxyUserEnrichmentProvider`.
#[napi(
    object,
    js_name = "ForwardProxyEnrichmentOptions",
    namespace = "experimental"
)]
pub struct ForwardProxyEnrichmentOptionsNapi {
    pub url: String,
    pub timeout_ms: Option<i64>,
    pub retries: Option<u32>,
    pub proxy_config: Option<ProxyConfig>,
}

/// **Experimental.** Resolves ID-list memberships through a Statsig forward proxy.
#[napi(
    js_name = "ForwardProxyUserEnrichmentProvider",
    namespace = "experimental"
)]
pub struct ForwardProxyUserEnrichmentProviderNapi {
    inner: Arc<ForwardProxyUserEnrichmentProvider>,
}

#[napi(namespace = "experimental")]
impl ForwardProxyUserEnrichmentProviderNapi {
    #[napi(factory)]
    pub fn create(sdk_key: String, options: ForwardProxyEnrichmentOptionsNapi) -> Result<Self> {
        let provider = ForwardProxyUserEnrichmentProvider::new(
            &sdk_key,
            ForwardProxyEnrichmentOptions {
                url: options.url,
                timeout_ms: options
                    .timeout_ms
                    .and_then(|value| u64::try_from(value).ok()),
                retries: options.retries,
                proxy_config: options.proxy_config.map(Into::into),
            },
        )
        .map_err(|error| Error::from_reason(error.to_string()))?;
        Ok(Self {
            inner: Arc::new(provider),
        })
    }
}

/// **Experimental.** Builds a `StatsigUser` whose ID-list memberships were
/// resolved through a forward proxy. `build()` returns a normal `StatsigUser`.
#[napi(js_name = "StatsigFPUserBuilder", namespace = "experimental")]
pub struct StatsigFPUserBuilderNapi {
    provider: Arc<ForwardProxyUserEnrichmentProvider>,
    user_data: UserData,
}

#[napi(namespace = "experimental")]
impl StatsigFPUserBuilderNapi {
    #[napi(constructor)]
    pub fn new(
        provider: &ForwardProxyUserEnrichmentProviderNapi,
        args: Option<StatsigUserArgs>,
    ) -> Self {
        let user = match args {
            Some(args) => StatsigUser::new(args),
            None => StatsigUser::new(empty_user_args()),
        };
        Self {
            provider: provider.inner.clone(),
            user_data: (*user.as_inner().data).clone(),
        }
    }

    #[napi]
    pub async fn build(&self) -> Result<StatsigUser> {
        let builder =
            StatsigFPUserBuilder::from_user_data(self.provider.clone(), self.user_data.clone());
        builder
            .build()
            .await
            .map(StatsigUser::from)
            .map_err(|error| Error::from_reason(error.to_string()))
    }
}

fn empty_user_args() -> StatsigUserArgs {
    StatsigUserArgs {
        user_id: None,
        custom_ids: None,
        email: None,
        ip: None,
        user_agent: None,
        country: None,
        locale: None,
        app_version: None,
        statsig_environment: None,
        custom: None,
        private_attributes: None,
    }
}
