#include <iostream>
#include <cassert>
#include <string>
#include <unordered_map>
#include <gtest/gtest.h>
#include "../src/statsig.h"

TEST(StatsigE2EUsageTest, Core_API) {
    const char* sdkKey = std::getenv("test_api_key");
    statsig_cpp_core::UserBuilder userBuilder;
    userBuilder.setUserID("123");
    statsig_cpp_core::User user = userBuilder.build();
    statsig_cpp_core::Statsig s = statsig_cpp_core::Statsig(sdkKey);
    s.initializeBlocking();
    bool pass = s.checkGate(user, "test_public");
    EXPECT_EQ(pass,1);
    s.shutdownBlocking();
}