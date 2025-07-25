pub mod bench_core;
pub mod bench_legacy;

use bench_core::BenchCore;
use bench_legacy::BenchLegacy;

#[tokio::main]
pub async fn main() {
    // let variant = std::env::var("SDK_VARIANT");

    BenchCore::run().await;
    // if variant == "core" {
    // } else if variant == "legacy" {
    //     BenchLegacy::run();
    // } else {
    //     panic!("Invalid SDK variant: {}", variant);
    // }
}
