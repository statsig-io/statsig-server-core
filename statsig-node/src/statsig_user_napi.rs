use napi_derive::napi;
use sigstat::StatsigUser as StatsigUserActual;

#[napi]
pub struct StatsigUser {
    inner: StatsigUserActual,
}

#[napi]
impl StatsigUser {
    #[napi(constructor)]
    pub fn new(user_id: String) -> Self {
        Self {
            inner: StatsigUserActual::with_user_id(user_id),
        }
    }

    pub fn as_inner(&self) -> &StatsigUserActual {
        &self.inner
    }
}
