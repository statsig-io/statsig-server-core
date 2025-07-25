pub mod bench_core;
pub mod bench_legacy;

use bench_core::BenchCore;
use bench_legacy::BenchLegacy;

#[tokio::main]
pub async fn main() {
    let variant = std::env::var("SDK_VARIANT");

    if variant.as_deref() == Ok("core") {
        BenchCore::run().await;
    } else if variant.as_deref() == Ok("legacy") {
        BenchLegacy::run().await;
    } else {
        panic!("Invalid SDK variant: {:?}", variant);
    }
}
