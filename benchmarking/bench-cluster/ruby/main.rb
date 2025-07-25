sdk_variant = ENV["SDK_VARIANT"]

if sdk_variant == "core"
    require_relative "bench_core"
    BenchCore.run
else
    require_relative "bench_legacy"
    BenchLegacy.run
end
