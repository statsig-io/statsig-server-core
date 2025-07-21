FROM mcr.microsoft.com/dotnet/sdk:8.0 AS builder
WORKDIR /app
COPY . .
RUN dotnet add package Statsig --prerelease
RUN dotnet add package Statsig.Dotnet --prerelease
RUN dotnet build -c Release

FROM mcr.microsoft.com/dotnet/runtime:8.0
WORKDIR /app
COPY --from=builder /app/bin/Release/net8.0 .
ENTRYPOINT ["dotnet", "bench.dll"]