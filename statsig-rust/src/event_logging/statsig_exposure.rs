use crate::event_logging::statsig_event_internal::StatsigEventInternal;

pub trait StatsigExposure {
    fn make_dedupe_key(&self) -> String;
    fn to_internal_event(self) -> StatsigEventInternal;
}
