<?php

namespace Statsig;

class ProxyConfig
{
    /**
     * The hostname or IP address of the proxy server
     * @var string|null
     */
    public ?string $proxyHost;

    /**
     * The port number of the proxy server
     * @var int
     */
    public int $proxyPort;

    /**
     * Authentication credentials for the proxy (e.g., "username:password")
     * @var string|null
     */
    public ?string $proxyAuth;

    /**
     * The protocol to use for the proxy connection (e.g., "http", "https", "socks5")
     * @var string|null
     */
    public ?string $proxyProtocol;

    public function __construct(
        ?string $proxyHost = null,
        int $proxyPort = 0,
        ?string $proxyAuth = null,
        ?string $proxyProtocol = null
    ) {
        $this->proxyHost = $proxyHost;
        $this->proxyPort = $proxyPort;
        $this->proxyAuth = $proxyAuth;
        $this->proxyProtocol = $proxyProtocol;
    }
}
