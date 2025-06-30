use crate::{
    dyn_value, log_d, log_e, unwrap_or_return_with, user::StatsigUserInternal, DynamicValue,
};
use std::sync::{Arc, RwLock};

use super::{dynamic_string::DynamicString, evaluator_context::EvaluatorContext};

pub struct CountryLookup;

pub struct CountryLookupData {
    country_codes: Vec<String>,
    ip_ranges: Vec<i64>,
}

lazy_static::lazy_static! {
    static ref COUNTRY_LOOKUP_DATA: Arc<RwLock<Option<CountryLookupData>>> = Arc::from(RwLock::from(None));
    static ref IP: String = "ip".to_string();
}

const TAG: &str = "CountryLookup";
const UNINITIALIZED_REASON: &str = "CountryLookupNotLoaded";

pub trait UsizeExt {
    fn post_inc(&mut self) -> Self;
}

impl UsizeExt for usize {
    fn post_inc(&mut self) -> Self {
        let was = *self;
        *self += 1;
        was
    }
}

impl CountryLookup {
    pub fn load_country_lookup() {
        match COUNTRY_LOOKUP_DATA.read() {
            Ok(lock) => {
                if lock.is_some() {
                    log_d!(TAG, "Country Lookup already loaded");
                    return;
                }
            }
            Err(e) => {
                log_e!(TAG, "Failed to acquire read lock on country lookup: {}", e);
                return;
            }
        }

        let bytes = include_bytes!("../../resources/ip_supalite.table");

        let mut raw_code_lookup: Vec<String> = vec![];
        let mut country_codes: Vec<String> = vec![];
        let mut ip_ranges: Vec<i64> = vec![];

        let mut i = 0;

        while i < bytes.len() {
            let c1 = bytes[i.post_inc()] as char;
            let c2 = bytes[i.post_inc()] as char;

            raw_code_lookup.push(format!("{c1}{c2}"));

            if c1 == '*' {
                break;
            }
        }

        let longs = |index: usize| bytes[index] as i64;

        let mut last_end_range = 0_i64;
        while (i + 1) < bytes.len() {
            let mut count: i64 = 0;
            let n1 = longs(i.post_inc());
            if n1 < 240 {
                count = n1;
            } else if n1 == 242 {
                let n2 = longs(i.post_inc());
                let n3 = longs(i.post_inc());
                count = n2 | (n3 << 8);
            } else if n1 == 243 {
                let n2 = longs(i.post_inc());
                let n3 = longs(i.post_inc());
                let n4 = longs(i.post_inc());
                count = n2 | (n3 << 8) | (n4 << 16);
            }

            last_end_range += count * 256;

            let cc = bytes[i.post_inc()] as usize;
            ip_ranges.push(last_end_range);
            country_codes.push(raw_code_lookup[cc].clone())
        }

        let country_lookup = CountryLookupData {
            country_codes,
            ip_ranges,
        };

        match COUNTRY_LOOKUP_DATA.write() {
            Ok(mut lock) => {
                *lock = Some(country_lookup);
                log_d!(TAG, " Successfully Loaded");
            }
            Err(e) => {
                log_e!(TAG, "Failed to acquire write lock on country_lookup: {}", e);
            }
        }
    }

    pub fn get_value_from_ip(
        user: &StatsigUserInternal,
        field: &Option<DynamicString>,
        evaluator_context: &mut EvaluatorContext,
    ) -> Option<DynamicValue> {
        let unwrapped_field = match field {
            Some(f) => f.value.as_str(),
            _ => return None,
        };

        if unwrapped_field != "country" {
            return None;
        }

        let ip = match user.get_user_value(&Some(DynamicString::from(IP.to_string()))) {
            Some(v) => match &v.string_value {
                Some(s) => &s.value,
                _ => return None,
            },
            None => return None,
        };

        Self::lookup(ip, evaluator_context)
    }

    fn lookup(ip_address: &str, evaluator_context: &mut EvaluatorContext) -> Option<DynamicValue> {
        let parts: Vec<&str> = ip_address.split('.').collect();
        if parts.len() != 4 {
            return None;
        }

        let lock = unwrap_or_return_with!(COUNTRY_LOOKUP_DATA.read().ok(), || {
            evaluator_context.result.override_reason = Some(UNINITIALIZED_REASON);
            log_e!(TAG, "Failed to acquire read lock on country lookup");
            None
        });

        let country_lookup_data = unwrap_or_return_with!(lock.as_ref(), || {
            evaluator_context.result.override_reason = Some(UNINITIALIZED_REASON);
            log_e!(TAG, "Failed to load country lookup. Did you disable CountryLookup or did not wait for country lookup to init. Check StatsigOptions configuration");
            None
        });

        let nums: Vec<Option<i64>> = parts.iter().map(|&x| x.parse().ok()).collect();
        if let (Some(n0), Some(n1), Some(n2), Some(n3)) = (nums[0], nums[1], nums[2], nums[3]) {
            let ip_number = (n0 * 256_i64.pow(3)) + (n1 << 16) + (n2 << 8) + n3;
            return Self::lookup_numeric(ip_number, country_lookup_data);
        }

        None
    }

    fn lookup_numeric(
        ip_address: i64,
        country_lookup_data: &CountryLookupData,
    ) -> Option<DynamicValue> {
        let index = Self::binary_search(ip_address, country_lookup_data);
        let cc = country_lookup_data.country_codes[index].clone();
        if cc == "--" {
            return None;
        }
        Some(dyn_value!(cc))
    }

    fn binary_search(value: i64, country_lookup_data: &CountryLookupData) -> usize {
        let mut min = 0;
        let mut max = country_lookup_data.ip_ranges.len();

        while min < max {
            let mid = (min + max) >> 1;
            if country_lookup_data.ip_ranges[mid] <= value {
                min = mid + 1;
            } else {
                max = mid;
            }
        }

        min
    }
}
