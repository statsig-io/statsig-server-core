#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub proxy_host: Option<String>,
    pub proxy_port: Option<u16>,
    pub proxy_auth: Option<String>,     // e.g., "username:password"
    pub proxy_protocol: Option<String>, // e.g., "http", "socks5", "https"
    pub ca_cert_path: Option<String>,   // Path to a PEM bundle for custom TLS roots
}
