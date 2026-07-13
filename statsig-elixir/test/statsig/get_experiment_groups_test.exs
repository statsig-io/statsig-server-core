defmodule Statsig.GetExperimentGroupsTest do
  use ExUnit.Case, async: false

  alias Statsig.Test.MockScrapi
  alias Statsig.Options

  @eval_proj_path Path.expand("../../../statsig-rust/tests/data/eval_proj_dcs.json", __DIR__)
  @eval_proj_data File.read!(@eval_proj_path)

  setup do
    {:ok, mock} = MockScrapi.start_link(@eval_proj_data)

    options = %Options{
      specs_url: mock.specs_url,
      disable_all_logging: true,
      output_log_level: "warn"
    }

    {:ok, _pid} = Statsig.start_link("secret-key", options)
    Statsig.initialize()

    on_exit(fn ->
      Statsig.shutdown()

      if pid = Process.whereis(Statsig) do
        Process.exit(pid, :normal)
      end

      MockScrapi.stop(mock)
    end)

    :ok
  end

  test "returns the groups for a known experiment" do
    {:ok, result} = Statsig.get_experiment_groups("test_experiment_no_targeting")

    assert result.is_experiment_active == true

    groups_by_name =
      Map.new(result.groups, fn group -> {group.group_name, group} end)

    # Only the experiment group rules are returned (the layerAssignment rule is excluded).
    assert Enum.sort(Map.keys(groups_by_name)) == ["Control", "Test", "Test2"]
    assert groups_by_name["Control"].return_value == %{"value" => "control"}
    assert groups_by_name["Control"].rule_id == "54QJztEPRLXK7ZCvXeY9q4"
    assert groups_by_name["Control"].id_type == "userID"
    assert groups_by_name["Test"].return_value == %{"value" => "test_1"}
    assert groups_by_name["Test2"].return_value == %{"value" => "test_2"}
  end

  test "returns ExperimentGroupsResult and ExperimentGroup structs" do
    {:ok, result} = Statsig.get_experiment_groups("test_experiment_no_targeting")

    assert match?(%Statsig.ExperimentGroupsResult{}, result)
    assert Enum.all?(result.groups, &match?(%Statsig.ExperimentGroup{}, &1))
  end

  test "returns nil active state for an unknown experiment" do
    assert {:ok, %Statsig.ExperimentGroupsResult{is_experiment_active: nil, groups: []}} =
             Statsig.get_experiment_groups("nonexistent_experiment")
  end

  test "returns nil active state for a dynamic config" do
    assert {:ok, %Statsig.ExperimentGroupsResult{is_experiment_active: nil, groups: []}} =
             Statsig.get_experiment_groups("test_max_dynamic_config_size_again")
  end

  test "returns the groups for an inactive experiment" do
    # test_switchback has isActive: false; groups are still returned along with the flag.
    {:ok, result} = Statsig.get_experiment_groups("test_switchback")

    assert result.is_experiment_active == false

    # Only the experiment group rules are returned (non-group rules are excluded).
    assert result.groups |> Enum.map(& &1.group_name) |> Enum.sort() == ["Control", "Test"]
  end
end
