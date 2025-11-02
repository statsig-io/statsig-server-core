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
        public void SpecAdapterConfig_JsonSerialization_SerializesCorrectly()
        {
            var config = new SpecAdapterConfig(
                SpecAdapterType.NetworkGrpcWebsocket,
                5000,
                "https://api.statsig.com/v1/specs",
                AuthenticationMode.Tls,
                "/path/to/ca.crt",
                "/path/to/client.crt",
                "/path/to/client.key",
                "statsig.com");

            var json = JsonConvert.SerializeObject(config, new JsonSerializerSettings
            {
                NullValueHandling = NullValueHandling.Ignore
            });

            Assert.Contains("\"spec_adapter_type\":\"network_grpc_websocket\"", json);
            Assert.Contains("\"spec_adapter_url\":\"https://api.statsig.com/v1/specs\"", json);
            Assert.Contains("\"spec_adapter_init_timeout_ms\":5000", json);
            Assert.Contains("\"spec_adapter_authentication_mode\":\"tls\"", json);
            Assert.Contains("\"spec_adapter_ca_cert_path\":\"/path/to/ca.crt\"", json);
            Assert.Contains("\"spec_adapter_client_cert_path\":\"/path/to/client.crt\"", json);
            Assert.Contains("\"spec_adapter_client_key_path\":\"/path/to/client.key\"", json);
            Assert.Contains("\"spec_adapter_domain_name\":\"statsig.com\"", json);
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

        private static object? GetInternalField(object obj, string fieldName)
        {
            var field = obj.GetType().GetField(fieldName, BindingFlags.NonPublic | BindingFlags.Instance);
            return field?.GetValue(obj);
        }
    }
}
