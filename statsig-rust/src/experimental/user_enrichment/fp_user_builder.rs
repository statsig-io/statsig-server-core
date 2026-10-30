use super::{EnrichmentError, IDListEnrichmentProvider, IdListLookups};
use crate::evaluation::evaluator::id_list_lookup_id;
use crate::hashing::HashUtil;
use crate::user::unit_id::UnitID;
use crate::user::user_data::{UserData, UserDataMap, UserDataStringMap};
use crate::{DynamicValue, StatsigUser, StatsigUserBuilder};
use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

static LOOKUP_HASHER: OnceLock<HashUtil> = OnceLock::new();

pub struct StatsigFPUserBuilder {
    inner: StatsigUserBuilder,
    enricher: Arc<dyn IDListEnrichmentProvider>,
}

impl StatsigFPUserBuilder {
    pub fn new_with_enricher(enricher: Arc<dyn IDListEnrichmentProvider>) -> Self {
        Self {
            inner: StatsigUserBuilder {
                user_id: None,
                custom_ids: None,
                email: None,
                ip: None,
                user_agent: None,
                country: None,
                locale: None,
                app_version: None,
                custom: None,
                private_attributes: None,
                statsig_environment: None,
                id_list_memberships: None,
            },
            enricher,
        }
    }

    /// Seeds the builder from already-deserialized user data. Used by the
    /// language bindings, which cross the FFI boundary with a `UserData` JSON
    /// payload rather than individual setter calls.
    pub fn from_user_data(enricher: Arc<dyn IDListEnrichmentProvider>, data: UserData) -> Self {
        let mut builder = Self::new_with_enricher(enricher);
        builder = builder.user_id(data.user_id.and_then(dyn_to_unit_id));
        if let Some(custom_ids) = data.custom_ids {
            let mapped: Vec<(String, UnitID)> = custom_ids
                .into_iter()
                .filter_map(|(key, value)| dyn_to_unit_id(value).map(|id| (key, id)))
                .collect();
            builder = builder.custom_ids(Some(mapped));
        }
        builder = builder
            .email(dyn_to_string(data.email))
            .ip(dyn_to_string(data.ip))
            .user_agent(dyn_to_string(data.user_agent))
            .country(dyn_to_string(data.country))
            .locale(dyn_to_string(data.locale))
            .app_version(dyn_to_string(data.app_version))
            .custom(data.custom)
            .private_attributes(data.private_attributes);
        if let Some(environment) = data.statsig_environment {
            let mapped = environment
                .into_iter()
                .filter_map(|(key, value)| dyn_to_string(Some(value)).map(|id| (key, id)))
                .collect();
            builder = builder.statsig_environment(Some(mapped));
        }
        builder
    }

    pub fn user_id(mut self, user_id: Option<impl Into<UnitID>>) -> Self {
        self.inner = self.inner.user_id(user_id);
        self
    }

    pub fn custom_ids<K, U, I>(mut self, custom_ids: Option<I>) -> Self
    where
        I: IntoIterator<Item = (K, U)>,
        K: Into<String>,
        U: Into<UnitID>,
    {
        self.inner = self.inner.custom_ids(custom_ids);
        self
    }

    pub fn email(mut self, email: Option<String>) -> Self {
        self.inner = self.inner.email(email);
        self
    }

    pub fn ip(mut self, ip: Option<String>) -> Self {
        self.inner = self.inner.ip(ip);
        self
    }

    pub fn user_agent(mut self, user_agent: Option<String>) -> Self {
        self.inner = self.inner.user_agent(user_agent);
        self
    }

    pub fn country(mut self, country: Option<String>) -> Self {
        self.inner = self.inner.country(country);
        self
    }

    pub fn locale(mut self, locale: Option<String>) -> Self {
        self.inner = self.inner.locale(locale);
        self
    }

    pub fn app_version(mut self, app_version: Option<String>) -> Self {
        self.inner = self.inner.app_version(app_version);
        self
    }

    pub fn statsig_environment(mut self, statsig_environment: Option<UserDataStringMap>) -> Self {
        self.inner = self.inner.statsig_environment(statsig_environment);
        self
    }

    pub fn custom_from_str_map(mut self, custom: Option<UserDataStringMap>) -> Self {
        self.inner = self.inner.custom_from_str_map(custom);
        self
    }

    pub fn custom(mut self, custom: Option<UserDataMap>) -> Self {
        self.inner = self.inner.custom(custom);
        self
    }

    pub fn private_attributes_from_str_map(
        mut self,
        private_attributes: Option<UserDataStringMap>,
    ) -> Self {
        self.inner = self
            .inner
            .private_attributes_from_str_map(private_attributes);
        self
    }

    pub fn private_attributes(mut self, private_attributes: Option<UserDataMap>) -> Self {
        self.inner = self.inner.private_attributes(private_attributes);
        self
    }

    /// One enrichment round trip. `Ok` means memberships were written (including
    /// an empty map). `Err` is the provider error; no user is returned.
    pub async fn build(self) -> Result<StatsigUser, EnrichmentError> {
        let mut user = self.inner.build();
        let lookups = collect_id_list_lookups(&user);

        let memberships = self.enricher.resolve_id_list_memberships(&lookups).await?;
        Arc::make_mut(&mut user.data).id_list_memberships = Some(memberships);
        Ok(user)
    }
}

fn collect_id_list_lookups(user: &StatsigUser) -> IdListLookups {
    let hashing = LOOKUP_HASHER.get_or_init(HashUtil::new);
    let mut lookups = HashSet::new();

    if let Some(user_id) = user
        .data
        .user_id
        .as_ref()
        .and_then(|value| value.string_value.as_ref())
    {
        lookups.insert(id_list_lookup_id(hashing, &user_id.value));
    }

    if let Some(custom_ids) = &user.data.custom_ids {
        for value in custom_ids.values() {
            if let Some(id) = &value.string_value {
                lookups.insert(id_list_lookup_id(hashing, &id.value));
            }
        }
    }

    let mut lookups: Vec<_> = lookups.into_iter().collect();
    lookups.sort_unstable();
    lookups
}

fn dyn_to_unit_id(value: DynamicValue) -> Option<UnitID> {
    // DynamicValue caches numeric interpretations even for strings. The original
    // JSON variant is the authority for preserving IDs such as "00123".
    match value.json_value {
        serde_json::Value::String(s) => Some(UnitID::String(s)),
        serde_json::Value::Number(n) => n
            .as_i64()
            .map(UnitID::Int)
            .or_else(|| n.as_f64().map(UnitID::Float)),
        _ => None,
    }
}

fn dyn_to_string(value: Option<DynamicValue>) -> Option<String> {
    value.and_then(|dynamic| dynamic.string_value.map(|s| s.value.to_string()))
}

// A provider-local TTL membership cache can wrap
// `IDListEnrichmentProvider` without changing this builder's API.
