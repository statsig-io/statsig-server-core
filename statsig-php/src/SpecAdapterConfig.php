<?php

namespace Statsig;

class SpecAdapterConfig
{
    public const TYPE_NETWORK_GRPC_WEBSOCKET = 'network_grpc_websocket';
    public const TYPE_NETWORK_HTTP = 'network_http';
    public const TYPE_DATA_STORE = 'data_store';

    public const AUTH_NONE = 'none';
    public const AUTH_TLS = 'tls';
    public const AUTH_MTLS = 'mtls';

    public const DEFAULT_INIT_TIMEOUT_MS = 3000;

    /**
     * One of the TYPE_* constants.
     * @var string
     */
    public string $adapterType;

    /**
     * Adapter endpoint, such as the Forward Proxy address for gRPC websocket.
     * @var string|null
     */
    public ?string $specsUrl;

    /**
     * Initial sync timeout in milliseconds. Null uses DEFAULT_INIT_TIMEOUT_MS.
     * @var int|null
     */
    public ?int $initTimeoutMs;

    /**
     * One of the AUTH_* constants. Used by the gRPC websocket adapter.
     * @var string|null
     */
    public ?string $authenticationMode;

    /**
     * CA certificate path for tls and mtls.
     * @var string|null
     */
    public ?string $caCertPath;

    /**
     * Client certificate path for mtls.
     * @var string|null
     */
    public ?string $clientCertPath;

    /**
     * Client private key path for mtls.
     * @var string|null
     */
    public ?string $clientKeyPath;

    /**
     * TLS server name used when verifying the certificate.
     * @var string|null
     */
    public ?string $domainName;

    public function __construct(
        string $adapterType,
        ?string $specsUrl = null,
        ?int $initTimeoutMs = null,
        ?string $authenticationMode = null,
        ?string $caCertPath = null,
        ?string $clientCertPath = null,
        ?string $clientKeyPath = null,
        ?string $domainName = null
    ) {
        $this->adapterType = $adapterType;
        $this->specsUrl = $specsUrl;
        $this->initTimeoutMs = $initTimeoutMs;
        $this->authenticationMode = $authenticationMode;
        $this->caCertPath = $caCertPath;
        $this->clientCertPath = $clientCertPath;
        $this->clientKeyPath = $clientKeyPath;
        $this->domainName = $domainName;
    }
}
