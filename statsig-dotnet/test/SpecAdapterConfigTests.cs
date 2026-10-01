using Xunit;
using System.Reflection;
using Newtonsoft.Json;

#nullable enable

namespace Statsig.Tests
{
    public class SpecAdapterConfigTests
    {
        [Fact]
        public void SpecAdapterConfig_Constructor_WithAllParameters_SetsAllProperties()
        {
            var adapterType = SpecAdapterType.NetworkGrpcWebsocket;
            var initTimeoutMs = 5000;
            var specsUrl = "https://api.statsig.com/v1/specs";
            var authenticationMode = AuthenticationMode.Tls;
            var caCertPath = "/path/to/ca.crt";
            var clientCertPath = "/path/to/client.crt";
            var clientKeyPath = "/path/to/client.key";
            var domainName = "statsig.com";

            var config = new SpecAdapterConfig(
                adapterType,
                initTimeoutMs,
                specsUrl,
                authenticationMode,
                caCertPath,
                clientCertPath,
                clientKeyPath,
                domainName);

            Assert.Equal(adapterType, config.AdapterType);
            Assert.Equal(initTimeoutMs, config.InitTimeoutMs);
            Assert.Equal(specsUrl, config.SpecsUrl);
            Assert.Equal(authenticationMode, config.AuthenticationMode);
            Assert.Equal(caCertPath, config.CaCertPath);
            Assert.Equal(clientCertPath, config.ClientCertPath);
            Assert.Equal(clientKeyPath, config.ClientKeyPath);
            Assert.Equal(domainName, config.DomainName);
        }

        [Fact]
        public void SpecAdapterConfig_Constructor_WithMinimalParameters_UsesDefaults()
        {
            var adapterType = SpecAdapterType.DataStore;
            var config = new SpecAdapterConfig(adapterType);

            Assert.Equal(adapterType, config.AdapterType);
            Assert.Equal(SpecAdapterConfig.DEFAULT_INIT_TIMEOUT_MS, config.InitTimeoutMs);
            Assert.Null(config.SpecsUrl);
            Assert.Null(config.AuthenticationMode);
            Assert.Null(config.CaCertPath);
            Assert.Null(config.ClientCertPath);
            Assert.Null(config.ClientKeyPath);
            Assert.Null(config.DomainName);
        }

        [Fact]
        public void StatsigOptionsBuilder_SetSpecAdapterConfig_SetsConfigCorrectly()
        {
            var builder = new StatsigOptionsBuilder();
            var specAdapterConfig = new SpecAdapterConfig(
                SpecAdapterType.NetworkGrpcWebsocket,
                5000,
                "https://api.statsig.com/v1/specs",
                AuthenticationMode.Tls);

            var result = builder.SetSpecAdapterConfig(specAdapterConfig);

            Assert.Same(builder, result);
            Assert.Same(specAdapterConfig, GetInternalField(builder, "specAdapterConfig"));
        }

        [Theory]
        [InlineData(SpecAdapterType.NetworkHttp)]
        [InlineData(SpecAdapterType.NetworkGrpcWebsocket)]
        public void StatsigOptionsBuilder_WithSpecAdapter_SerializesFfiKeys(string adapterType)
        {
            // The StatsigOptionsBuilder is serialized (with NullValueHandling.Ignore)
            // and handed to statsig_options_create_from_data. The flattened
            // spec_adapter_* keys MUST match StatsigOptionsData in
            // statsig-ffi/src/statsig_options_c.rs exactly, or the core silently
            // drops the values (serde ignores unknown fields).
            var builder = new StatsigOptionsBuilder()
                .SetSpecAdapterConfig(new SpecAdapterConfig(
                    adapterType,
                    5000,
                    "http://localhost:50051",
                    AuthenticationMode.Mtls,
                    "/certs/ca.pem",
                    "/certs/client.pem",
                    "/certs/client.key",
                    "proxy.example.com"));

            var json = JsonConvert.SerializeObject(builder, new JsonSerializerSettings
            {
                NullValueHandling = NullValueHandling.Ignore
            });

            Assert.Contains($"\"spec_adapter_type\":\"{adapterType}\"", json);
            // Regression guard: the core expects spec_adapter_url, not
            // spec_adapter_specs_url.
            Assert.Contains("\"spec_adapter_url\":\"http://localhost:50051\"", json);
            Assert.DoesNotContain("spec_adapter_specs_url", json);
            Assert.Contains("\"spec_adapter_init_timeout_ms\":5000", json);
            Assert.Contains("\"spec_adapter_authentication_mode\":\"mtls\"", json);
            Assert.Contains("\"spec_adapter_ca_cert_path\":\"/certs/ca.pem\"", json);
            Assert.Contains("\"spec_adapter_client_cert_path\":\"/certs/client.pem\"", json);
            Assert.Contains("\"spec_adapter_client_key_path\":\"/certs/client.key\"", json);
            Assert.Contains("\"spec_adapter_domain_name\":\"proxy.example.com\"", json);
        }

        [Fact]
        public void StatsigOptionsBuilder_WithGrpcSpecAdapter_OmitsUnsetTlsKeys()
        {
            // A minimal gRPC websocket config omits the TLS keys so the core
            // defaults them to None.
            var builder = new StatsigOptionsBuilder()
                .SetSpecAdapterConfig(new SpecAdapterConfig(
                    SpecAdapterType.NetworkGrpcWebsocket,
                    3000,
                    "http://localhost:50051"));

            var json = JsonConvert.SerializeObject(builder, new JsonSerializerSettings
            {
                NullValueHandling = NullValueHandling.Ignore
            });

            Assert.Contains("\"spec_adapter_type\":\"network_grpc_websocket\"", json);
            Assert.Contains("\"spec_adapter_url\":\"http://localhost:50051\"", json);
            Assert.DoesNotContain("spec_adapter_authentication_mode", json);
            Assert.DoesNotContain("spec_adapter_ca_cert_path", json);
            Assert.DoesNotContain("spec_adapter_domain_name", json);
        }

        [Fact]
        public void StatsigOptionsBuilder_BuildWithGrpcSpecAdapter_Succeeds()
        {
            // End-to-end through the native FFI: building options parses (but does
            // not start) the spec adapter, so this succeeds as long as the
            // flattened JSON is accepted by statsig_options_create_from_data.
            using var options = new StatsigOptionsBuilder()
                .SetSpecAdapterConfig(new SpecAdapterConfig(
                    SpecAdapterType.NetworkGrpcWebsocket,
                    3000,
                    "http://localhost:50051"))
                .Build();

            Assert.NotNull(options);
        }

        private static object? GetInternalField(object obj, string fieldName)
        {
            var field = obj.GetType().GetField(fieldName, BindingFlags.NonPublic | BindingFlags.Instance);
            return field?.GetValue(obj);
        }
    }
}
