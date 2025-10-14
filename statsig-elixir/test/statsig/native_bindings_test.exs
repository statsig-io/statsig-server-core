defmodule Statsig.NativeBindingsTest do
  use ExUnit.Case

  alias Statsig.DynamicConfig
  alias Statsig.NativeBindings
  alias Statsig.Options
  alias Statsig.User

  defp new do
    ref = NativeBindings.new("", %Options{disable_network: true}, Statsig.get_system_info())
    NativeBindings.initialize(ref)

    on_exit(fn -> NativeBindings.shutdown(ref) end)

    ref
  end

  defp user(id) do
    %User{user_id: id}
  end

  describe "override_gate/4" do
    test "overrides a feature gate for a user, but not others" do
      ref = new()
      user = user("user1")

      refute NativeBindings.check_gate(ref, "my_gate", user, nil)

      NativeBindings.override_gate(ref, "my_gate", true, user.user_id)

      assert NativeBindings.check_gate(ref, "my_gate", user, nil)
      refute NativeBindings.check_gate(ref, "my_gate", user("user2"), nil)

      NativeBindings.remove_gate_override(ref, "my_gate", user.user_id)

      refute NativeBindings.check_gate(ref, "my_gate", user, nil)
    end

    test "overrides a feature gate for all users" do
      ref = new()
      user1 = user("user1")
      user2 = user("user2")

      refute NativeBindings.check_gate(ref, "my_gate", user1, nil)
      refute NativeBindings.check_gate(ref, "my_gate", user2, nil)

      NativeBindings.override_gate(ref, "my_gate", true, nil)

      assert NativeBindings.check_gate(ref, "my_gate", user1, nil)
      assert NativeBindings.check_gate(ref, "my_gate", user2, nil)

      NativeBindings.remove_gate_override(ref, "my_gate", nil)

      refute NativeBindings.check_gate(ref, "my_gate", user1, nil)
      refute NativeBindings.check_gate(ref, "my_gate", user2, nil)
    end
  end

  describe "override_dynamic_config/4" do
    test "overrides a dynamic config for a user, but not others" do
      ref = new()
      user1 = user("user1")
      user2 = user("user2")
      overridden_value = %{"a" => 1, "b" => "test", "c" => true}

      NativeBindings.override_dynamic_config(ref, "my_config", overridden_value, user1.user_id)

      config1 = NativeBindings.get_dynamic_config(ref, "my_config", user1, nil)
      assert config1.value == overridden_value

      config2 = NativeBindings.get_dynamic_config(ref, "my_config", user2, nil)
      assert config2.value != overridden_value

      NativeBindings.remove_dynamic_config_override(ref, "my_config", user1.user_id)

      config3 = NativeBindings.get_dynamic_config(ref, "my_config", user1, nil)
      assert config3.value != overridden_value
    end

    test "overrides a dynamic config for all users" do
      ref = new()
      user1 = user("user1")
      user2 = user("user2")
      overridden_value = %{"x" => 42, "nested" => %{"key" => "value"}}

      NativeBindings.override_dynamic_config(ref, "my_config", overridden_value, nil)

      config1 = NativeBindings.get_dynamic_config(ref, "my_config", user1, nil)
      assert config1.value == overridden_value

      config2 = NativeBindings.get_dynamic_config(ref, "my_config", user2, nil)
      assert config2.value == overridden_value

      NativeBindings.remove_dynamic_config_override(ref, "my_config", nil)

      config3 = NativeBindings.get_dynamic_config(ref, "my_config", user1, nil)
      assert config3.value != overridden_value
    end

    test "handles complex nested structures" do
      ref = new()
      user1 = user("user1")

      complex_value = %{
        "string" => "hello",
        "number" => 42,
        "float" => 3.14,
        "bool" => true,
        "null" => nil,
        "array" => [1, 2, 3, "four"],
        "nested" => %{
          "deep" => %{
            "value" => "works!"
          }
        }
      }

      NativeBindings.override_dynamic_config(ref, "complex_config", complex_value, user1.user_id)

      config = NativeBindings.get_dynamic_config(ref, "complex_config", user1, nil)
      assert %DynamicConfig{name: "complex_config", value: ^complex_value} = config
    end
  end

  describe "override_experiment/4" do
    test "overrides an experiment for a user, but not others" do
      ref = new()
      user1 = user("user1")
      user2 = user("user2")
      overridden_value = %{"exp_param" => "test_value", "count" => 100}

      NativeBindings.override_experiment(ref, "my_experiment", overridden_value, user1.user_id)

      exp1 = NativeBindings.get_experiment(ref, "my_experiment", user1, nil)
      assert exp1.value == overridden_value

      exp2 = NativeBindings.get_experiment(ref, "my_experiment", user2, nil)
      assert exp2.value != overridden_value

      NativeBindings.remove_experiment_override(ref, "my_experiment", user1.user_id)

      exp3 = NativeBindings.get_experiment(ref, "my_experiment", user1, nil)
      assert exp3.value != overridden_value
    end

    test "overrides an experiment for all users" do
      ref = new()
      user1 = user("user1")
      user2 = user("user2")
      overridden_value = %{"param" => "global"}

      NativeBindings.override_experiment(ref, "my_experiment", overridden_value, nil)

      exp1 = NativeBindings.get_experiment(ref, "my_experiment", user1, nil)
      assert exp1.value == overridden_value

      exp2 = NativeBindings.get_experiment(ref, "my_experiment", user2, nil)
      assert exp2.value == overridden_value

      NativeBindings.remove_experiment_override(ref, "my_experiment", nil)

      exp3 = NativeBindings.get_experiment(ref, "my_experiment", user1, nil)
      assert exp3.value != overridden_value
    end
  end

  describe "override_layer/4" do
    test "overrides a layer for a user, but not others" do
      ref = new()
      user1 = user("user1")
      user2 = user("user2")
      overridden_value = %{"layer_param" => "custom", "value" => 42}

      NativeBindings.override_layer(ref, "my_layer", overridden_value, user1.user_id)

      layer1 = NativeBindings.get_layer(ref, "my_layer", user1, nil)
      assert NativeBindings.layer_get(layer1, "layer_param", "default") == "custom"

      layer2 = NativeBindings.get_layer(ref, "my_layer", user2, nil)
      assert NativeBindings.layer_get(layer2, "layer_param", "default") == "default"

      NativeBindings.remove_layer_override(ref, "my_layer", user1.user_id)

      layer3 = NativeBindings.get_layer(ref, "my_layer", user1, nil)
      assert NativeBindings.layer_get(layer3, "layer_param", "default") == "default"
    end

    test "overrides a layer for all users" do
      ref = new()
      user1 = user("user1")
      user2 = user("user2")
      overridden_value = %{"param" => "overridden"}

      NativeBindings.override_layer(ref, "my_layer", overridden_value, nil)

      layer1 = NativeBindings.get_layer(ref, "my_layer", user1, nil)
      assert NativeBindings.layer_get(layer1, "param", "default") == "overridden"

      layer2 = NativeBindings.get_layer(ref, "my_layer", user2, nil)
      assert NativeBindings.layer_get(layer2, "param", "default") == "overridden"

      NativeBindings.remove_layer_override(ref, "my_layer", nil)

      layer3 = NativeBindings.get_layer(ref, "my_layer", user1, nil)
      assert NativeBindings.layer_get(layer3, "param", "default") == "default"
    end
  end

  describe "remove_all_overrides/1" do
    test "removes all overrides at once" do
      ref = new()
      user1 = user("user1")

      # Set multiple overrides
      NativeBindings.override_gate(ref, "gate1", true, user1.user_id)
      NativeBindings.override_dynamic_config(ref, "config1", %{"key" => "value"}, user1.user_id)
      NativeBindings.override_experiment(ref, "exp1", %{"param" => "value"}, user1.user_id)

      # Verify overrides are active
      assert NativeBindings.check_gate(ref, "gate1", user1, nil) == true

      config = NativeBindings.get_dynamic_config(ref, "config1", user1, nil)
      assert config.value == %{"key" => "value"}

      # Remove all overrides
      NativeBindings.remove_all_overrides(ref)

      # Verify all overrides are gone
      refute NativeBindings.check_gate(ref, "gate1", user1, nil)

      config2 = NativeBindings.get_dynamic_config(ref, "config1", user1, nil)
      assert config2.value != %{"key" => "value"}
    end
  end
end
