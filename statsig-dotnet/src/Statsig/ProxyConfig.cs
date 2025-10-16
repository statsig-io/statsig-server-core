namespace Statsig
{
    /// <summary>
    /// Configuration for HTTP proxy settings
    /// </summary>
    public class ProxyConfig
    {
        /// <summary>
        /// The hostname or IP address of the proxy server
        /// </summary>
        public string? ProxyHost { get; set; }

        /// <summary>
        /// The port number of the proxy server
        /// </summary>
        public int ProxyPort { get; set; }

        /// <summary>
        /// Authentication credentials for the proxy (e.g., "username:password")
        /// </summary>
        public string? ProxyAuth { get; set; }

        /// <summary>
        /// The protocol to use for the proxy connection (e.g., "http", "https", "socks5")
        /// </summary>
        public string? ProxyProtocol { get; set; }

        public ProxyConfig()
        {
        }

        public ProxyConfig(string? proxyHost, int proxyPort, string? proxyAuth = null, string? proxyProtocol = null)
        {
            ProxyHost = proxyHost;
            ProxyPort = proxyPort;
            ProxyAuth = proxyAuth;
            ProxyProtocol = proxyProtocol;
        }
    }
}

