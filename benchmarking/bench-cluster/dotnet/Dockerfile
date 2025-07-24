FROM mcr.microsoft.com/dotnet/sdk:8.0 AS builder
WORKDIR /app
COPY . .

RUN apt-get update && apt-get install -y jq
RUN LATEST_VERSION=$(curl -s https://api.nuget.org/v3-flatcontainer/statsig.dotnet/index.json \
      | jq -r '.versions[]' \
      | grep -Ev '(-rc|-alpha|-preview)' \
      | sort -V | tail -n1) \
  && echo "🎯 Latest allowed version: $LATEST_VERSION" \
  && sed -i -E "s|Version=\"0.0.0-placeholder\"|Version=\"${LATEST_VERSION}\"|g" bench.csproj \
  && echo "📝 Updated .csproj:" \
  && grep Statsig.Dotnet bench.csproj

RUN dotnet restore
RUN dotnet build -c Release

FROM mcr.microsoft.com/dotnet/runtime:8.0
WORKDIR /app
COPY --from=builder /app/bin/Release/net8.0 .
ENTRYPOINT ["dotnet", "bench.dll"]