using Xunit;

namespace Statsig.Tests
{
    public class ProxyConfigTests
    {
        [Fact]
        public void StatsigOptions_ProxyConfigBeforeOtherOptions_WorksCorrectly()
        {
            var proxyConfig = new ProxyConfig("proxy.example.com", 8080);

            var options = new StatsigOptionsBuilder()
                .SetProxyConfig(proxyConfig)
                .SetSpecsURL("https://custom.statsig.com/v1/download_config_specs")
                .SetLogEventURL("https://custom.statsig.com/v1/log_event")
                .Build();

            Assert.NotNull(options);
        }

        [Fact]
        public void StatsigOptionsBuilder_FluentInterface_AllMethodsChain()
        {
            var proxyConfig = new ProxyConfig("proxy.example.com", 8080);

            var builder = new StatsigOptionsBuilder()
                .SetSpecsURL("https://api.statsig.com")
                .SetLogEventURL("https://events.statsig.com")
                .SetEnvironment("test")
                .SetInitTimeoutMs(10000)
                .SetProxyConfig(proxyConfig)
                .SetDisableNetwork(false)
                .SetDisableAllLogging(false);

            Assert.NotNull(builder);
            var options = builder.Build();
            Assert.NotNull(options);
        }

        [Fact]
        public void Statsig_WithFullProxyConfig_CreatesAndInitializes()
        {
            var proxyConfig = new ProxyConfig
            {
                ProxyHost = "proxy.example.com",
                ProxyPort = 8080,
                ProxyAuth = "user:password",
                ProxyProtocol = "http"
            };

            var options = new StatsigOptionsBuilder()
                .SetProxyConfig(proxyConfig)
                .Build();

            using var statsig = new Statsig("test-sdk-key", options);

            Assert.NotNull(statsig);
        }
    }
}

