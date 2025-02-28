defmodule StatsigTest do
  use ExUnit.Case
  doctest Statsig

  test "initialize statsig and check gate" do
    # Initialize Statsig with a test SDK key
    IO.puts("\n=== Starting Statsig Test ===")
    sdk_key = System.get_env("test_api_key")
    if sdk_key == nil do
      IO.puts("This test is for Statsig internal testing")
      :ok
    end
    statsig_options = %StatsigOptions{enable_id_lists: true}
    IO.puts("Initializing with SDK key: #{sdk_key}")
    {:ok,_} = Statsig.start_link(sdk_key, statsig_options)
    client = Statsig.get_statsig_instance();

    # Create a test user
    user = %StatsigUser{
      user_id: "test_user_123"
    }
    Statsig.initialize()

    # Check a feature gate
    IO.puts("\nChecking gate 'test_gate'...")
    check_gate = Statsig.check_gate("test_public", user)
    assert check_gate
    feature_gate = Statsig.get_feature_gate("test_public", user)
    assert feature_gate.value
    assert feature_gate.name == "test_public"
    config = Statsig.get_config("test_custom_config", user)
    assert is_binary(config.value)
    param_value = DynamicConfig.get_param_value(config, "header_text")
    assert param_value == "old user test"
    IO.inspect(config)
    experiment = Statsig.get_experiment("test_custom_config", user)
    param_value = Experiment.get_param_value(experiment, "header_text")
    assert param_value == "old user test"
    # config = Statsig.get_config("test_public", user)
    # IO.inspect(config, label: "Gate check result")
    # experiment = Statsig.get_experiment("test_public", user)
    # IO.inspect(experiment, label: "Gate check result")
    # config = Statsig.get_feature_gate("test_public", user)
    # IO.inspect(config, label: "Gate check result")
    Statsig.shutdown()
    # Assert the result is a boolean
    assert check_gate
    IO.puts("=== Test Complete ===\n")
  end
end
