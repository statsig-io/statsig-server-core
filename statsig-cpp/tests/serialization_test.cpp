#include "../src/statsig.h"
#include <cassert>
#include <gtest/gtest.h>
#include <iostream>
#include <string>
#include <unordered_map>
using statsig_cpp_core::allowed_primitive;
using statsig_cpp_core::allowed_type;

TEST(Serialization, User) {
  std::string json_str_1 = R"({
        "userID": "test_user",
        "customIDs": {"custom_key": "custom_value"},
        "email": "test_user@example.com",
        "ip": "192.168.1.1",
        "userAgent": "Mozilla/5.0",
        "country": "US",
        "locale": "en-US",
        "privateAttributes": {
            "private": ["123"]
        },
        "custom": {
            "height": "1"
        }
    })";
  json j_1 = json::parse(json_str_1);
  statsig_cpp_core::UserBuilder user_1;
  from_json(j_1, user_1);
  std::cout << "User id: " << j_1["userID"] << std::endl;
  EXPECT_EQ(user_1.userID, "test_user");
  EXPECT_EQ(user_1.customIDs.value()["custom_key"], "custom_value");
  EXPECT_EQ(user_1.email, "test_user@example.com");
  EXPECT_EQ(user_1.ip, "192.168.1.1");
  EXPECT_EQ(user_1.userAgent, "Mozilla/5.0");
  EXPECT_EQ(user_1.country, "US");
  EXPECT_EQ(user_1.locale, "en-US");
  std::cout << "Private attribute is array: "
            << j_1["privateAttributes"]["private"].is_array() << std::endl;
  allowed_primitive &val =
      std::get<allowed_primitive>(user_1.custom.value()["height"]);
  EXPECT_EQ(std::get<std::string>(val), "1");
  allowed_type exp_private_values = std::vector<allowed_primitive>{"123"};
  allowed_type d = user_1.privateAttribute.value()["private"];

  EXPECT_EQ(user_1.privateAttribute.value()["private"], exp_private_values);

  std::string json_str_2 = R"({
        "userID": "test_user_2",
        "customIDs": {"custom_key": "custom_value_2"}
    })";
  json j_2 = json::parse(json_str_2);
  statsig_cpp_core::UserBuilder user_2;
  from_json(j_2, user_2);
  EXPECT_EQ(user_2.userID, "test_user_2");
  EXPECT_EQ(user_2.customIDs.value()["custom_key"], "custom_value_2");
  EXPECT_EQ(user_2.email, std::nullopt);
  EXPECT_EQ(user_2.ip, std::nullopt);
  EXPECT_EQ(user_2.userAgent, std::nullopt);
  EXPECT_EQ(user_2.country, std::nullopt);
  EXPECT_EQ(user_2.locale, std::nullopt);

  json j_1_out;
  to_json(j_1_out, user_1);
  EXPECT_EQ(j_1_out.dump(),
            "{\"appVersion\":null,\"country\":\"US\",\"custom\":{\"height\":"
            "\"1\"},\"customIDs\":{\"custom_key\":\"custom_value\"},\"email\":"
            "\"test_user@example.com\",\"ip\":\"192.168.1.1\",\"locale\":\"en-"
            "US\",\"privateAttributes\":{\"private\":[\"123\"]},\"userAgent\":"
            "\"Mozilla/5.0\",\"userID\":\"test_user\"}");

    statsig_cpp_core::UserBuilder user_obj = statsig_cpp_core::UserBuilder();
    user_obj.build();
}

TEST(Serialization, StatsigOptionsExposureDedupeMaxKeys) {
  // The serialized JSON is passed to statsig_options_create_from_data, which
  // deserializes exposure_dedupe_max_keys into the core StatsigOptions.
  statsig_cpp_core::StatsigOptionsBuilder builder;
  builder.exposure_dedupe_max_keys = 50000;

  json j;
  to_json(j, builder);
  EXPECT_EQ(j["exposure_dedupe_max_keys"], 50000);

  // When unset, the field serializes to null so the core falls back to its
  // default (100,000).
  statsig_cpp_core::StatsigOptionsBuilder unset_builder;
  json j_unset;
  to_json(j_unset, unset_builder);
  EXPECT_TRUE(j_unset["exposure_dedupe_max_keys"].is_null());
}

TEST(Serialization, DynamicConfig) {
  std::string json_str = R"({
        "name": "example_config",
        "value": {
            "param1": "value1",
            "param2": 42
        },
        "rule_id": "rule_123",
        "id_type": "userID",
        "details": {
            "lcut": 1627847261,
            "received_at": 1627847265,
            "reason": "Network:Recognized"
        }
    })";
  json j = json::parse(json_str);
  statsig_cpp_core::DynamicConfig config;
  from_json(j, config);
  EXPECT_EQ(config.name, "example_config");
  EXPECT_EQ(config.value["param1"], "value1");
  EXPECT_EQ(config.value["param2"], 42);
  EXPECT_EQ(config.rule_id, "rule_123");
  EXPECT_EQ(config.id_type, "userID");
  EXPECT_EQ(config.details.lcut.value(), 1627847261);
  EXPECT_EQ(config.details.receivedAt.value(), 1627847265);
  EXPECT_EQ(config.details.reason, "Network:Recognized");

  // Serialize back to JSON
  json j_out;
  to_json(j_out, config);
  EXPECT_EQ(
      j_out.dump(),
      "{\"details\":{\"lcut\":1627847261,\"reason\":\"Network:Recognized\","
      "\"receivedAt\":1627847265},\"id_type\":\"userID\",\"name\":\"example_"
      "config\","
      "\"rule_id\":\"rule_123\",\"value\":{\"param1\":\"value1\",\"param2\":42}"
      "}");
}

TEST(Serialization, Layer) {
  std::string json_str_1 = R"({
        "name": "example_layer",
        "__value": {
            "param1": "value1",
            "param2": 42
        },
        "rule_id": "rule_123",
        "id_type": "userID",
        "group_name": "group1",
        "allocated_experiment_name": "experiment_1",
        "is_experiment_active": true,
        "details": {
            "lcut": 1627847261,
            "received_at": 1627847265,
            "reason": "Network:Recognized"
        }
    })";

  json j_1 = json::parse(json_str_1);
  statsig_cpp_core::Layer layer_1;
  from_json(j_1, layer_1);
  EXPECT_EQ(layer_1.rule_id, "rule_123");
  EXPECT_EQ(layer_1.id_type, "userID");
  EXPECT_EQ(layer_1.value["param1"], "value1");
  EXPECT_EQ(layer_1.value["param2"], 42);
  EXPECT_EQ(layer_1.group_name.value(), "group1");
  EXPECT_EQ(layer_1.allocated_experiment_name, "experiment_1");
  EXPECT_EQ(layer_1.details.lcut.value(), 1627847261);
  EXPECT_EQ(layer_1.details.receivedAt.value(), 1627847265);
  EXPECT_EQ(layer_1.details.reason, "Network:Recognized");

  std::string json_str_2 = R"({
        "name": "example_layer",
        "__value": {
            "param1": "value1",
            "param2": 42
        },
        "rule_id": "rule_123",
        "id_type": "userID",
        "is_experiment_active": true,
        "details": {
            "lcut": 1627847261,
            "received_at": 1627847265,
            "reason": "Network:Recognized"
        }
    })";

  json j_2 = json::parse(json_str_2);
  statsig_cpp_core::Layer layer_2;
  from_json(j_2, layer_2);
  EXPECT_EQ(layer_2.rule_id, "rule_123");
  EXPECT_EQ(layer_2.id_type, "userID");
  EXPECT_EQ(layer_2.value["param1"], "value1");
  EXPECT_EQ(layer_2.value["param2"], 42);
  EXPECT_FALSE(layer_2.allocated_experiment_name.has_value());
  EXPECT_EQ(layer_2.details.lcut.value(), 1627847261);
  EXPECT_EQ(layer_2.details.receivedAt.value(), 1627847265);
  EXPECT_EQ(layer_2.details.reason, "Network:Recognized");
  EXPECT_FALSE(layer_2.group_name.has_value());
}

TEST(Serialization, FeatureGate) {
  std::string json_str = R"({
        "name": "example_gate",
        "value": true,
        "rule_id": "rule_123",
        "id_type": "userID",
        "details": {
            "lcut": 1627847261,
            "received_at": 1627847265,
            "reason": "Network:Recognized"
        }
    })";
  json j = json::parse(json_str);
  statsig_cpp_core::FeatureGate gate;
  from_json(j, gate);
  EXPECT_EQ(gate.name, "example_gate");
  EXPECT_EQ(gate.value, true);
  EXPECT_EQ(gate.rule_id, "rule_123");
  EXPECT_EQ(gate.id_type, "userID");
  EXPECT_EQ(gate.details.lcut.value(), 1627847261);
  EXPECT_EQ(gate.details.receivedAt.value(), 1627847265);
  EXPECT_EQ(gate.details.reason, "Network:Recognized");
}

TEST(Serialization, Experiment) {
  std::string json_str = R"({
        "name": "example_experiment",
        "value": {
            "param1": "value1",
            "param2": 42
        },
        "rule_id": "rule_123",
        "id_type": "userID",
        "group_name": "group1",
        "details": {
            "lcut": 1627847261,
            "received_at": 1627847265,
            "reason": "Network:Recognized"
        }
    })";
  json j = json::parse(json_str);
  statsig_cpp_core::Experiment experiment;
  from_json(j, experiment);
  EXPECT_EQ(experiment.name, "example_experiment");
  EXPECT_EQ(experiment.value["param1"], "value1");
  EXPECT_EQ(experiment.value["param2"], 42);
  EXPECT_EQ(experiment.rule_id, "rule_123");
  EXPECT_EQ(experiment.id_type, "userID");
  EXPECT_EQ(experiment.group_name.value(), "group1");
  EXPECT_EQ(experiment.details.lcut.value(), 1627847261);
  EXPECT_EQ(experiment.details.receivedAt.value(), 1627847265);
  EXPECT_EQ(experiment.details.reason, "Network:Recognized");
}

// The group-targeting getters (getExperimentByGroupName /
// getExperimentByGroupIdAdvanced) return the camelCase ExperimentRaw shape
// (ruleID / idType / groupName, and a null value when unrecognized), unlike
// the snake_case shape from the normal getExperiment path. from_json must
// parse both.
TEST(Serialization, ExperimentRawGroupShape) {
  std::string recognized = R"({
        "name": "test_experiment_no_targeting",
        "value": {"value": "control"},
        "ruleID": "54QJztEPRLXK7ZCvXeY9q4",
        "idType": "userID",
        "groupName": "Control",
        "isExperimentActive": true,
        "details": {
            "lcut": 1627847261,
            "received_at": 1627847265,
            "reason": "Network:Recognized"
        },
        "secondaryExposures": null
    })";
  statsig_cpp_core::Experiment exp(recognized);
  EXPECT_EQ(exp.name, "test_experiment_no_targeting");
  EXPECT_EQ(exp.value["value"].get<std::string>(), "control");
  EXPECT_EQ(exp.rule_id, "54QJztEPRLXK7ZCvXeY9q4");
  EXPECT_EQ(exp.id_type, "userID");
  EXPECT_EQ(exp.group_name.value(), "Control");
  EXPECT_EQ(exp.is_experiment_active, true);
  EXPECT_EQ(exp.details.reason, "Network:Recognized");

  // Unrecognized: null value, empty ruleID, null idType/groupName.
  std::string unrecognized = R"({
        "name": "test_experiment_no_targeting",
        "value": null,
        "ruleID": "",
        "idType": null,
        "groupName": null,
        "isExperimentActive": null,
        "details": {"reason": "Unrecognized"},
        "secondaryExposures": null
    })";
  statsig_cpp_core::Experiment empty(unrecognized);
  EXPECT_EQ(empty.name, "test_experiment_no_targeting");
  EXPECT_TRUE(empty.value.empty());
  EXPECT_EQ(empty.rule_id, "");
  EXPECT_EQ(empty.id_type, "");
  EXPECT_FALSE(empty.group_name.has_value());
  EXPECT_EQ(empty.is_experiment_active, false);
}