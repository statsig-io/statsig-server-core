defmodule Statsigelixir.MixProject do
  use Mix.Project

  def project do
    [
      app: :statsig_elixir,
      version: "0.0.6-beta.6",
      elixir: "~> 1.0",
      start_permanent: Mix.env() == :prod,
      description: description(),
      deps: deps(),
      package: package()
    ]
  end

  defp description() do
    "An performant elixir SDK for Statsig feature gates and experiments using Rustler"
  end


  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      extra_applications: [:logger]
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      # {:dep_from_hexpm, "~> 0.3.0"},
      # {:dep_from_git, git: "https://github.com/elixir-lang/my_dep.git", tag: "0.1.0"}
      {:rustler, "~> 0.22-rc"},
      {:rustler_precompiled, "~> 0.8"},
      {:ex_doc, "~> 0.27", only: :dev, runtime: false},
    ]
  end

  defp package() do
    [
      # These are the default files included in the package
      files: ~w(mix.exs LICENSE* README* lib native checksum-*.exs),
      licenses: ["ISC"],
      links: %{"GitHub" => "https://github.com/statsig-io/statsig-server-core"}
    ]
  end
end
