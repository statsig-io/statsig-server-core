defmodule StatsigTest do
  use ExUnit.Case
  doctest Statsig

  alias Statsig.Layer
  alias Statsig.Options
  alias Statsig.User
  alias Statsig.ExperimentEvaluationOptions
  alias Statsig.FeatureGateEvaluationOptions
  alias Statsig.LayerEvaluationOptions
  alias Statsig.DynamicConfigEvaluationOptions

  @tag :skip
  test "Example usage test" do
    # Initialize Statsig with a test SDK key
    IO.puts("\n=== Starting Statsig Test ===")
    sdk_key = System.get_env("test_api_key")

    if sdk_key == nil do
      IO.puts("This test is for Statsig internal testing")
      :ok
    end

    statsig_options = %Options{enable_id_lists: true, output_log_level: "debug"}
    IO.puts("Initializing with SDK key: #{sdk_key}")
    initres = Statsig.start_link(sdk_key, statsig_options)
    IO.inspect(initres)
    # Create a test user
    user = %User{
      user_id: "test_user_123",
      email: "test@email.com",
      custom_ids: %{
        "a" => "v"
      }
    }

    Statsig.initialize()

    # Check a feature gate
    IO.puts("\nChecking gate 'test_gate'...")
    {:ok, _check_gate} = Statsig.check_gate("test_public", user)

    {:ok, check_gate} =
      Statsig.check_gate("test_public", user, %FeatureGateEvaluationOptions{
        disable_exposure_logging: true
      })

    assert check_gate
    {:ok, feature_gate} = Statsig.get_feature_gate("test_public", user)
    assert feature_gate.value
    assert feature_gate.name == "test_public"

    {:ok, config} =
      Statsig.get_dynamic_config("test_custom_config", user, %DynamicConfigEvaluationOptions{
        disable_exposure_logging: true
      })

    IO.inspect(config)
    assert is_map(config.value)

    IO.puts("\nGetting a subfield in param value")
    param_value = config.value["header_text"]
    assert param_value == "old user test"
    IO.inspect(config)

    {:ok, experiment} =
      Statsig.get_experiment("test_custom_config", user, %ExperimentEvaluationOptions{
        disable_exposure_logging: true
      })

    param_value = experiment.value["header_text"]
    assert param_value == "old user test"

    {:ok, layer} =
      Statsig.get_layer("layer_with_many_params", user, %LayerEvaluationOptions{
        disable_exposure_logging: true
      })

    {:ok, a_string_value} = Layer.get(layer, "a_string", "default")
    IO.inspect(a_string_value)
    assert a_string_value == "layer"
    {:ok, an_object_value} = Layer.get(layer, "an_object", "default")
    assert an_object_value == "{\"value\":\"layer_default\"}"
    IO.inspect(an_object_value)

    {:ok, default_value} = Layer.get(layer, "invalid_param", "default")
    assert default_value == "default"

    IO.puts("\nLog event")
    Statsig.log_event(user, "test_event", "string_value", %{"metadata_1" => "value"})
    result = Statsig.log_event(user, "test_event", 1, %{"metadata_1" => "value"})
    IO.inspect(result)
    Statsig.shutdown()
    # Assert the result is a boolean
    IO.puts("=== Test Complete ===\n")
  end
end
