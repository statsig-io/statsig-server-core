use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum StatsigErr {
    CustomError(String),

    // Specs Adapter
    SpecsListenerNotSet,
    SpecsListenerLockFailure,
    SpecsAdapterNetworkFailure,
    SpecsAdapterLockFailure,
    BackgroundTaskLockFailure
}

impl Display for StatsigErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StatsigErr::CustomError(msg) => write!(f, "{}", msg),
            StatsigErr::SpecsListenerNotSet => write!(f, "No SpecsUpdateListener found"),
            StatsigErr::SpecsListenerLockFailure => write!(f, "Failed to acquire mutex lock for SpecsUpdateListener"),
            StatsigErr::SpecsAdapterLockFailure => write!(f, "Failed to acquire lock"),
            StatsigErr::SpecsAdapterNetworkFailure => write!(f, "Specs adapter network failure"),
            StatsigErr::BackgroundTaskLockFailure => write!(f, "Specs adapter failed to acquire background task mutex lock"),
        }
    }
}
