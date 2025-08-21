FROM elixir:1.15-slim AS builder
RUN apt-get update && apt-get install -y \
    libc6 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY mix.exs mix.lock ./
RUN mix local.hex --force && \
    mix local.rebar --force && \
    mix deps.get --only prod
COPY main.ex bench-core.ex bench-legacy.ex ./
RUN mix compile
RUN mix release

FROM elixir:1.15-slim
WORKDIR /app
COPY --from=builder /app/_build/dev/rel/bench_cluster ./
ENV MIX_ENV=prod

CMD ["/app/bin/bench_cluster", "start"]
# CMD ["sh", "-c", "while true; do sleep 1000; done"]