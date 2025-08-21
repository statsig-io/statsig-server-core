defmodule Main do
  use Application

  def start(_type, _args) do
    main(System.argv())
    {:ok, self()}
  end

  def main(_args) do
    sdk_variant = System.get_env("SDK_VARIANT")

    case sdk_variant do
      nil ->
        raise "SDK_VARIANT is not set"

      "core" ->
        BenchCore.main([])

      "legacy" ->
        BenchLegacy.main([])

      invalid ->
        raise "Invalid SDK_VARIANT: #{invalid}"
    end

    System.halt(0)
  end
end
