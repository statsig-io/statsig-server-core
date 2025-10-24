#include "options.h"
#include "libstatsig_ffi.h"
#include <iostream>
#include <nlohmann/json.hpp>

namespace statsig_cpp_core {
StatsigOptions StatsigOptionsBuilder::build() {
  json j = *this;
  std::string serialized = j.dump();
  uint64_t ref = statsig_options_create_from_data(serialized.c_str());
  return StatsigOptions(ref);
}
StatsigOptions::~StatsigOptions() {
  if (ref != 0) {
    statsig_options_release(ref);
  }
}
} // namespace statsig_cpp_core