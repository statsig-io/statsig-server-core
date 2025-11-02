FROM mcr.microsoft.com/dotnet/sdk:8.0 AS build

WORKDIR /app

COPY sr.csproj ./
RUN dotnet restore

COPY . ./
RUN dotnet build -c Release --no-restore
RUN dotnet publish -c Release --no-build -o /app/publish

# Runtime stage
FROM mcr.microsoft.com/dotnet/runtime:8.0

WORKDIR /app

COPY --from=build /app/publish .

RUN mkdir -p /shared-volume

ENTRYPOINT ["dotnet", "sr.dll"]
