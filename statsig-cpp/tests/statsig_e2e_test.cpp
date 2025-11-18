#include "../src/statsig.h"
#include <cassert>
#include <gtest/gtest.h>
#include <iostream>
#include <string>
#include <unordered_map>

TEST(StatsigE2EUsageTest, Core_API) {
  const char *sdkKey = std::getenv("test_api_key");
  statsig_cpp_core::StatsigOptionsBuilder optionsBuilder;
  optionsBuilder.specs_url = "https://api.statsig.com/v2/download_config_specs";
  optionsBuilder.output_log_level = "debug";
  statsig_cpp_core::UserBuilder userBuilder;
  userBuilder.setUserID("cpp-core-test-user");
  statsig_cpp_core::User user = userBuilder.build();
  userBuilder.setUserID("cpp-core-non-exposed-user");
  statsig_cpp_core::User nonExposedUser = userBuilder.build();
  statsig_cpp_core::Statsig s =
      statsig_cpp_core::Statsig(sdkKey, optionsBuilder.build());
  s.initializeBlocking();

  // Gate Check
  bool pass = s.checkGate(user, "test_public");
  EXPECT_EQ(pass, true);
  statsig_cpp_core::FeatureGate gate = s.getFeatureGate(user, "test_public");
  std::cout << "Gate: " << gate.name
            << ", Value: " << (gate.value ? "true" : "false")
            << ", RuleID: " << gate.rule_id << std::endl;
  EXPECT_EQ(gate.name, "test_public");
  EXPECT_EQ(gate.value, true);
  EXPECT_EQ(gate.details.reason,
            "Network:Recognized"); // Adjust this based on expected reason
  statsig_cpp_core::CheckGateOptions gateOptions;
  gateOptions.disableExposureLogging = true;
  statsig_cpp_core::FeatureGate nonExposedGate =
      s.getFeatureGate(nonExposedUser, "test_public", gateOptions);
  EXPECT_EQ(nonExposedGate.value, true);

  // Dynamic Config
  statsig_cpp_core::Experiment e =
      s.getExperiment(user, "experiment_with_many_params");
  EXPECT_EQ(e.name, "experiment_with_many_params");
  EXPECT_EQ(e.id_type, "userID");
  EXPECT_EQ(e.value["a_number"].get<double>(), 1);
  EXPECT_EQ(e.value["a_string"].get<std::string>(), "control");
  EXPECT_EQ(e.value["an_array"].get<std::vector<std::string>>()[0], "control");
  statsig_cpp_core::GetExperimentOptions experimentOptions;
  experimentOptions.disableExposureLogging = true;
  statsig_cpp_core::Experiment nonExposedExperiment = s.getExperiment(
      nonExposedUser, "experiment_with_many_params", experimentOptions);
  EXPECT_EQ(nonExposedExperiment.value["a_number"].get<double>(), 2);

  // TODO(xinli) Should safely get non-exist key
  // EXPECT_EQ(e.value["non_exist_key"].get<std::string>(), "");

  // Experiment
  statsig_cpp_core::DynamicConfig config = s.getDynamicConfig(user, "big_number");
  EXPECT_EQ(config.name, "big_number");
  EXPECT_EQ(config.id_type, "userID");
  EXPECT_EQ(config.value["foo"].get<double>(), 1e21);
  statsig_cpp_core::GetDynamicConfigOptions configOptions;
  configOptions.disableExposureLogging = true;
  statsig_cpp_core::DynamicConfig nonExposedConfig =
      s.getDynamicConfig(nonExposedUser, "big_number", configOptions);
  EXPECT_EQ(nonExposedConfig.value["foo"].get<double>(), 1e21);

  // Layer
  statsig_cpp_core::Layer layer = s.getLayer(user, "test_layer");
  std::cout << "Layer Name: " << std::endl; // layer.name is empty
  bool layer_bool = layer.get<bool>("another_param", false);
  statsig_cpp_core::GetLayerOptions layerOptions;
  layerOptions.disableExposureLogging = true;
  statsig_cpp_core::Layer nonExposedLayer =
      s.getLayer(nonExposedUser, "test_layer", layerOptions);
  bool nonExposedLayerBool = nonExposedLayer.get<bool>("another_param", false);

  s.shutdownBlocking();
}