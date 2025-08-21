defmodule BenchmarkResult do
  defstruct [
    :benchmark_name,
    :p99,
    :max,
    :min,
    :median,
    :avg,
    :spec_name,
    :sdk_type,
    :sdk_version
  ]

  def to_map(result) do
    %{
      "benchmarkName" => result.benchmark_name,
      "p99" => result.p99,
      "max" => result.max,
      "min" => result.min,
      "median" => result.median,
      "avg" => result.avg,
      "specName" => result.spec_name,
      "sdkType" => result.sdk_type,
      "sdkVersion" => result.sdk_version
    }
  end
end

defmodule BenchCore do
  @sdk_type "statsig-server-core-elixir"
  @scrapi_url "http://scrapi:8000"
  @iter_lite 1000
  @iter_heavy 10_000

  def main(_args) do
    sdk_version = get_sdk_version()

    IO.puts("Statsig Elixir Core (v#{sdk_version})")
    IO.puts("--------------------------------")

    spec_names = load_spec_names()

    options = %Statsig.Options{
      specs_url: "#{@scrapi_url}/v2/download_config_specs",
      log_event_url: "#{@scrapi_url}/v1/log_event"
    }

    statsig = Statsig.start_link("secret-ELIXIR_CORE", options)
    Statsig.initialize()

    results = []

    global_user = %Statsig.User{user_id: "global_user"}

    # Feature gates
    results =
      Enum.reduce(spec_names["feature_gates"], results, fn gate, acc ->
        acc
        |> benchmark(
          "check_gate",
          gate,
          @iter_heavy,
          fn -> Statsig.check_gate(gate, create_user()) end,
          sdk_version
        )
        |> benchmark(
          "check_gate_global_user",
          gate,
          @iter_heavy,
          fn -> Statsig.check_gate(gate, global_user) end,
          sdk_version
        )
        |> benchmark(
          "get_feature_gate",
          gate,
          @iter_heavy,
          fn -> Statsig.get_feature_gate(gate, create_user()) end,
          sdk_version
        )
        |> benchmark(
          "get_feature_gate_global_user",
          gate,
          @iter_heavy,
          fn -> Statsig.get_feature_gate(gate, global_user) end,
          sdk_version
        )
      end)

    # Dynamic configs
    results =
      Enum.reduce(spec_names["dynamic_configs"], results, fn config, acc ->
        acc
        |> benchmark(
          "get_dynamic_config",
          config,
          @iter_heavy,
          fn -> Statsig.get_config(config, create_user()) end,
          sdk_version
        )
        |> benchmark(
          "get_dynamic_config_global_user",
          config,
          @iter_heavy,
          fn -> Statsig.get_config(config, global_user) end,
          sdk_version
        )
      end)

    # Experiments
    results =
      Enum.reduce(spec_names["experiments"], results, fn experiment, acc ->
        acc
        |> benchmark(
          "get_experiment",
          experiment,
          @iter_heavy,
          fn -> Statsig.get_experiment(experiment, create_user()) end,
          sdk_version
        )
        |> benchmark(
          "get_experiment_global_user",
          experiment,
          @iter_heavy,
          fn -> Statsig.get_experiment(experiment, global_user) end,
          sdk_version
        )
      end)

    # Layers
    results =
      Enum.reduce(spec_names["layers"], results, fn layer, acc ->
        acc
        |> benchmark(
          "get_layer",
          layer,
          @iter_heavy,
          fn -> Statsig.get_layer(layer, create_user()) end,
          sdk_version
        )
        |> benchmark(
          "get_layer_global_user",
          layer,
          @iter_heavy,
          fn -> Statsig.get_layer(layer, global_user) end,
          sdk_version
        )
      end)

    # todo: uncomment once this is supported
    # Client initialize response
    # results =
    #   results
    #   |> benchmark(
    #     "get_client_initialize_response",
    #     "n/a",
    #     @iter_lite,
    #     fn -> Statsig.get_client_init_response_as_string(statsig, create_user()) end,
    #     sdk_version
    #   )
    #   |> benchmark(
    #     "get_client_initialize_response_global_user",
    #     "n/a",
    #     @iter_lite,
    #     fn -> Statsig.get_client_init_response_as_string(statsig, global_user) end,
    #     sdk_version
    #   )

    Statsig.shutdown()
    Process.sleep(1000)

    write_results(@sdk_type, sdk_version, results)
  end

  defp get_sdk_version do
    Application.spec(:statsig_elixir, :vsn)
    |> to_string()
  end

  defp load_spec_names do
    path = "/shared-volume/spec_names.json"

    # Wait for file to be available
    wait_for_file(path, 10)

    case File.read(path) do
      {:ok, content} -> Jason.decode!(content)
      {:error, reason} -> raise "Failed to read spec_names.json: #{reason}"
    end
  end

  defp wait_for_file(_path, 0), do: :ok

  defp wait_for_file(path, attempts) do
    if File.exists?(path) do
      :ok
    else
      Process.sleep(1000)
      wait_for_file(path, attempts - 1)
    end
  end

  defp create_user do
    user_id = "user_#{:rand.uniform(1_000_000)}"

    %Statsig.User{
      user_id: user_id,
      email: "user@example.com",
      ip: "127.0.0.1",
      locale: "en-US",
      app_version: "1.0.0",
      country: "US",
      user_agent:
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
      custom: %{"isAdmin" => false},
      private_attributes: %{"isPaid" => "nah"}
    }
  end

  defp benchmark(results, bench_name, spec_name, iterations, func, sdk_version) do
    durations =
      for _ <- 1..iterations do
        start = System.monotonic_time(:microsecond)
        func.()
        stop = System.monotonic_time(:microsecond)
        # Convert to milliseconds
        (stop - start) / 1000.0
      end

    durations = Enum.sort(durations)
    p99_index = trunc(iterations * 0.99)

    result = %BenchmarkResult{
      benchmark_name: bench_name,
      p99: Enum.at(durations, p99_index),
      max: List.last(durations),
      min: List.first(durations),
      median: Enum.at(durations, trunc(length(durations) / 2)),
      avg: Enum.sum(durations) / length(durations),
      spec_name: spec_name,
      sdk_type: @sdk_type,
      sdk_version: sdk_version
    }

    results = [result | results]

    IO.puts(
      "#{String.pad_trailing(bench_name, 30)} p99(#{:erlang.float_to_binary(result.p99, decimals: 4)}ms) max(#{:erlang.float_to_binary(result.max, decimals: 4)}ms) #{spec_name}"
    )

    results
  end

  defp write_results(sdk_type, sdk_version, results) do
    output = %{
      "sdkType" => sdk_type,
      "sdkVersion" => sdk_version,
      "results" => Enum.map(results, &BenchmarkResult.to_map/1)
    }

    out_path = "/shared-volume/#{sdk_type}-#{sdk_version}-results.json"
    File.write!(out_path, Jason.encode!(output, pretty: true))
  end
end
