defmodule MixProject do
  use Mix.Project

  def project do
    [
      app: :bench_cluster,
      version: "0.1.0",
      elixir: "~> 1.15",
      deps: deps(),
      elixirc_paths: ["main.ex", "bench-core.ex", "bench-legacy.ex"]
    ]
  end

  def application do
    [
      extra_applications: [:logger],
      included_applications: [:statsig_elixir],
      mod: {Main, []}
    ]
  end

  defp deps do
    [
      {:statsig_elixir, "~> 0.7.4-beta.2508200235"},
      {:jason, "~> 1.4"}
    ]
  end
end
