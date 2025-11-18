#include <chrono>
#include <gtest/gtest.h>
#include <iostream>
#include <string>
#include <thread>

#include "../src/statsig.h"

#if defined(__APPLE__)
#include <mach/mach.h>
size_t getCurrentRSS() {
  task_basic_info info;
  mach_msg_type_number_t count = TASK_BASIC_INFO_COUNT;
  if (task_info(mach_task_self(), TASK_BASIC_INFO,
                reinterpret_cast<task_info_t>(&info), &count) != KERN_SUCCESS) {
    return 0;
  }
  return info.resident_size / 1024; // in KB
}
#else
#include <fstream>
#include <unistd.h>
size_t getCurrentRSS() {
  std::ifstream statm("/proc/self/statm");
  size_t total = 0, resident = 0;
  statm >> total >> resident;
  long page_size_kb = sysconf(_SC_PAGE_SIZE) / 1024;
  return resident * page_size_kb;
}
#endif

TEST(StatsigMemoryTest, ContinuousCoreApiCalls) {
  const char *sdkKey = std::getenv("test_api_key");
  statsig_cpp_core::Statsig statsig = statsig_cpp_core::Statsig(sdkKey);
  statsig.initializeBlocking();
  statsig_cpp_core::UserBuilder userBuilder;
  userBuilder.setUserID("memory_safety_test_user");
  statsig_cpp_core::User user = userBuilder.build();

  const int iterations = 100;

  size_t initial_rss = getCurrentRSS();

  for (int i = 0; i < iterations; ++i) {
    statsig_cpp_core::User user = userBuilder.build();
    statsig_cpp_core::FeatureGate gateValue =
        statsig.getFeatureGate(user, "test_public");
    statsig_cpp_core::DynamicConfig config =
        statsig.getDynamicConfig(user, "example_config");
    statsig_cpp_core::Experiment experiment =
        statsig.getExperiment(user, "example_experiment");
    statsig_cpp_core::Layer layer =
        statsig.getLayer(user, "example_layer");
  }
  sleep(2); // need to wait for cleanup

  size_t final_rss = getCurrentRSS();
  statsig.shutdownBlocking();
  std::cout << "Initial RSS: " << initial_rss << " KB" << std::endl;
  EXPECT_LT(final_rss - initial_rss, 100)
      << "Possible memory leak detected: RSS increased too much.";
}