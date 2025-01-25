use std::sync::Arc;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use sigstat::{log_d, Statsig as StatsigActual};

use crate::statsig_options_napi::StatsigOptions;
use crate::statsig_user_napi::StatsigUser;

const TAG: &str = "StatsigNapi";

#[napi]
pub struct Statsig {
    inner: Arc<StatsigActual>,
}

#[napi]
impl Statsig {
    #[napi(constructor)]
    pub fn new(sdk_key: String, options: Option<StatsigOptions>) -> Self {
        log_d!(TAG, "StatsigNapi new");

        let options = options.map(|o| Arc::new(o.into()));

        Self {
            inner: Arc::new(StatsigActual::new(&sdk_key, options)),
        }
    }

    #[napi]
    pub async fn initialize(&self) -> Result<()> {
        self.inner
            .initialize()
            .await
            .expect("Error initializing Statsig");

        Ok(())
    }

    #[napi]
    pub async fn shutdown(&self) -> Result<()> {
        self.inner
            .shutdown()
            .await
            .expect("Error shutting Statsig down");

        Ok(())
    }

    #[napi]
    pub fn check_gate(&self, user: &StatsigUser, gate_name: String) -> bool {
        self.inner.check_gate(user.as_inner(), &gate_name)
    }

    // #[napi]
    // pub fn test_callback<T>(&self, callback: T)
    // where
    //     T: Fn(String) -> Result<()>,
    // {
    //     callback("hello".to_string()).unwrap();
    // }
    //
    // #[napi]
    // pub fn test_async_callback(&self, callback: JsFunction) -> Result<()> {
    //     let tsfn: ThreadsafeFunction<_, ErrorStrategy::CalleeHandled> = callback
    //         .create_threadsafe_function(0, |ctx| {
    //             ctx.env.create_string(ctx.value).map(|v| vec![v])
    //         })?;
    //
    //     self.inner
    //         .statsig_runtime
    //         .spawn("async_cb", |_| async move {
    //             tsfn.call(Ok("hello"), ThreadsafeFunctionCallMode::Blocking);
    //         });
    //
    //     Ok(())
    // }
}

impl Drop for Statsig {
    fn drop(&mut self) {
        log_d!(TAG, "StatsigNapi dropped");
    }
}

// use napi::bindgen_prelude::External;
// use napi_derive::napi;
// use sigstat::{Statsig, StatsigUser};

// #[napi]
// pub fn sum(a: i32, b: i32) -> i32 {
//     a + b
// }

// #[napi]
// pub struct Person {
//     pub name: String,
//     pub age: i32,
// }

// #[napi]
// impl Person {
//     pub fn new(name: String, age: i32) -> Self {
//         Person { name, age }
//     }
// }

// #[napi]
// pub fn get_statsig() -> External<Statsig> {
//     let sdk_key = "secret-IiDuNzovZ5z9x75BEjyZ4Y2evYa94CJ9zNtDViKBVdv";
//     let statsig = Statsig::new(sdk_key, None);

//     External::new(statsig)
// }

// #[napi]
// pub fn check_gate(statsig: External<Statsig>, gate_name: String) -> bool {
//     let user = StatsigUser::with_user_id("user-123".to_string());
//     let gate = statsig.check_gate(&user, &gate_name);
//     gate
// }
