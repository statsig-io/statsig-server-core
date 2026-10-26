defmodule Statsig.CmabExposureRuleIdTest do
  # The SDK runs as a named singleton process, so these tests cannot run
  # concurrently with other suites that start Statsig.
  use ExUnit.Case, async: false

  alias Statsig.Options
  alias Statsig.Test.MockScrapi
  alias Statsig.User

  @moduledoc """
  Elixir uses the typed experiment path (get_experiment), so these are
  controls for the automatic exposure built from a typed CMAB evaluation.
  """

  @cmab_dcs File.read!(
              Path.expand(
                "../../../statsig-rust/tests/data/dcs_cmab_exposure_rule_id.json",
                __DIR__
              )
            )

  setup do
    {:ok, mock_scrapi} = MockScrapi.start_link(@cmab_dcs, notify: self())

    options = %Options{
      specs_url: mock_scrapi.specs_url,
      log_event_url: mock_scrapi.log_event_url
    }

    {:ok, _pid} = Statsig.start_link("secret-key", options)
    Statsig.initialize()

    on_exit(fn ->
      Statsig.shutdown()

      if pid = Process.whereis(Statsig) do
        Process.exit(pid, :normal)
      end

      MockScrapi.stop(mock_scrapi)
    end)

    :ok
  end

  test "explore suffix is preserved in the automatic exposure" do
    {:ok, experiment} =
      Statsig.get_experiment("cmab_always_explore", %User{user_id: "typed-explore-user"})

    Statsig.flush()

    assert String.ends_with?(experiment.rule_id, ":explore")
    assert logged_rule_id() == experiment.rule_id
  end

  test "exploit rule id remains bare in the automatic exposure" do
    {:ok, experiment} =
      Statsig.get_experiment("cmab_never_explore", %User{user_id: "typed-exploit-user"})

    Statsig.flush()

    assert experiment.rule_id == "treatment-group-id"
    assert logged_rule_id() == experiment.rule_id
  end

  defp logged_rule_id do
    receive do
      {:mock_scrapi_request, "/v1/log_event", headers, body} ->
        exposures =
          body
          |> decompress(headers["content-encoding"])
          |> JSON.decode!()
          |> Map.get("events", [])
          |> Enum.filter(&(&1["eventName"] == "statsig::config_exposure"))

        case exposures do
          [] -> logged_rule_id()
          [exposure] -> exposure["metadata"]["ruleID"]
          many -> flunk("expected 1 config exposure, got #{length(many)}")
        end

      {:mock_scrapi_request, _path, _headers, _body} ->
        logged_rule_id()
    after
      5_000 -> flunk("no config exposure was logged")
    end
  end

  defp decompress(body, "zstd"), do: body |> :zstd.decompress() |> IO.iodata_to_binary()
  defp decompress(body, "gzip"), do: :zlib.gunzip(body)
  defp decompress(body, _), do: body
end
