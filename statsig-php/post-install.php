<?php

const OUTPUT_DIR = "resources";
const DOMAIN = "pubkey.statsig.com";
const VERSION = "0.9.4-rc.2509300113";

if (getenv('SKIP_STATSIG_POST_INSTALL') === 'true') {
    exit(0);
}

function get_os()
{
    $os = PHP_OS_FAMILY;

    if ($os === 'Darwin') {
        $os = 'macos';
    } else if ($os === 'Linux') {
        $os = 'linux';
    } else if ($os === 'Windows') {
        $os = 'windows';
    } else {
        return null;
    }

    return $os;
}

function get_system_info()
{
    $arch = strtolower(php_uname('m'));

    if ($arch === 'amd64' || $arch === 'x86_64') {
        $arch = 'x86_64';
    } else if ($arch === 'arm64' || $arch === 'aarch64') {
        $arch = 'aarch64';
    } else {
        echo "Unsupported architecture: $arch\n";
        exit(1);
    }

    $os = get_os();

    if ($os === null) {
        echo "Unsupported OS: $os\n";
        exit(1);
    }

    echo "\n-- System Info --\n";
    echo " OS: {$os}\n";
    echo " Arch: {$arch}\n";
    echo "-----------------\n";

    return [$os, $arch];
}

function ensure_bin_dir_exists()
{
    if (is_dir(OUTPUT_DIR)) {
        return;
    }

    if (!mkdir(OUTPUT_DIR, 0755, true)) {
        echo "Failed to create directory: " . OUTPUT_DIR . "\n";
        exit(1);
    }
}

function remove_existing_statsig_resources()
{
    $dir = OUTPUT_DIR;

    $files = scandir($dir);
    foreach ($files as $file) {
        if (strpos($file, "statsig_ffi") !== false) {
            unlink($dir . '/' . $file);
        }
    }
}

function isMusl($os)
{
    if ($os !== 'linux') {
        return false;
    }

    function isMuslFromFilesystem()
    {
        try {
            $output = file_get_contents('/usr/bin/ldd');
            return strpos($output, 'musl') !== false;
        } catch (Exception $_) {
            return null;
        }
    }

    function isMuslFromChildProcess()
    {
        try {
            $output = shell_exec('ldd --version');
            return strpos($output, 'musl') !== false;
        } catch (Exception $_) {
            return false;
        }
    }

    $musl = isMuslFromFilesystem();
    if ($musl === null) {
        $musl = isMuslFromChildProcess();
    }

    return $musl === true;
}


function download_binary($system_info)
{
    $binary_map = [
        "macos-aarch64" => "statsig-ffi-" . VERSION . "-aarch64-apple-darwin.zip",
        "macos-x86_64" => "statsig-ffi-" . VERSION . "-x86_64-apple-darwin.zip",

        "linux-aarch64" => "statsig-ffi-" . VERSION . "-centos7-aarch64-unknown-linux-gnu.zip",
        "linux-x86_64" => "statsig-ffi-" . VERSION . "-centos7-x86_64-unknown-linux-gnu.zip",

        "linux-aarch64-musl" => "statsig-ffi-" . VERSION . "-alpine-aarch64-unknown-linux-musl.zip",
        "linux-x86_64-musl" => "statsig-ffi-" . VERSION . "-alpine-x86_64-unknown-linux-musl.zip",
    ];

    $system_tag = $system_info[0] . "-" . $system_info[1];
    if (isMusl($system_info[0])) {
        $system_tag .= "-musl";
    }

    $binary_file = $binary_map[$system_tag];

    if ($binary_file === null) {
        echo "No binary found for: {$system_tag}\n";
        exit(1);
    }

    $url = "https://github.com/statsig-io/statsig-php-core/releases/download/" . VERSION . "/" . $binary_file;

    echo "\n-- Downloading Statsig FFI Binary --\n";
    echo " Url: $url\n";
    echo " Output Path: " . OUTPUT_DIR . "/" . $binary_file . "\n";
    echo "-----------------------------------\n";

    $output_path = OUTPUT_DIR . "/" . $binary_file;

    file_put_contents($output_path, file_get_contents($url));

    return $output_path;
}

function unzip_binary($zip_file_path)
{
    echo "\n-- Unzipping Statsig FFI Binary --\n";
    echo " Input Path: $zip_file_path\n";

    $zip = new ZipArchive();
    if ($zip->open($zip_file_path) === TRUE) {
        for ($i = 0; $i < $zip->numFiles; $i++) {
            $filename = $zip->getNameIndex($i);
            if (in_array($filename, ['libstatsig_ffi.dylib', 'statsig_ffi.dll', 'libstatsig_ffi.so'])) {
                $zip->extractTo(OUTPUT_DIR, $filename);
                echo " Output Path: " . OUTPUT_DIR . "/" . $filename . "\n";
            }
            if (in_array($filename, ['libstatsig_ffi.dylib.sig', 'statsig_ffi.dll.sig', 'libstatsig_ffi.so.sig'])) {
                $zip->extractTo(OUTPUT_DIR, $filename);
                echo " Output Path: " . OUTPUT_DIR . "/" . $filename . "\n";
            }
        }
        $zip->close();
    } else {
        echo "Failed to open zip file\n";
    }

    if (!unlink($zip_file_path)) {
        echo "Failed to delete $zip_file_path\n";
    }
    echo "-----------------------------------\n";
}

function download_header()
{
    $url = "https://github.com/statsig-io/statsig-php-core/releases/download/" . VERSION . "/statsig_ffi.h";

    echo "\n-- Downloading Statsig FFI Header --\n";
    echo " Url: $url\n";
    echo " Output Path: " . OUTPUT_DIR . "/statsig_ffi.h\n";
    echo "-----------------------------------\n";

    $output_path = OUTPUT_DIR . "/statsig_ffi.h";
    file_put_contents($output_path, file_get_contents($url));
}

function download_public_key()
{
    echo "\n-- Downloading Statsig Public Key --\n";
    echo " Domain: " . DOMAIN . "\n";
    echo " Output Path: " . OUTPUT_DIR . "/public.pem\n";
    echo "-----------------------------------\n";

    $records = dns_get_record(DOMAIN, DNS_TXT);

    if ($records === false) {
        echo "No TXT records found.\n";
        return;
    }

    $allTxt = "";

    for ($i = count($records) - 1; $i >= 0; $i--) {
        $record = $records[$i];
        if (isset($record['entries']) && is_array($record['entries'])) {
            $txtValue = implode('', $record['entries']);
        } else {
            $txtValue = $record['txt'];
        }
        $allTxt .= str_replace('"', '', $txtValue);
    }

    $allTxt = trim(str_replace('"', '', $allTxt));
    $keyBody = chunk_split($allTxt, 64, "\n");

    $pem = "-----BEGIN PUBLIC KEY-----\n" .
        $keyBody .
        "-----END PUBLIC KEY-----\n";

    $output_path = OUTPUT_DIR . "/public.pem";
    file_put_contents($output_path, $pem);
}


function ensure_header_file_exists()
{
    $header_file = OUTPUT_DIR . "/statsig_ffi.h";

    if (!file_exists($header_file)) {
        echo "❌ Required header file statsig_ffi.h not found in resources directory\n";
        return false;
    } else {
        echo "✅ Header file statsig_ffi.h found in resources directory\n";
        return true;
    }
}

function get_binary_name($system_info)
{
    $binary_name = "libstatsig_ffi.so";
    if ($system_info[0] === "macos") {
        $binary_name = "libstatsig_ffi.dylib";
    } else if ($system_info[0] === "windows") {
        $binary_name = "statsig_ffi.dll";
    }
    return $binary_name;
}

function ensure_binary_file_exists($binary_name)
{
    $binary_file = OUTPUT_DIR . "/" . $binary_name;

    if (!file_exists($binary_file)) {
        echo "❌ Required binary file $binary_name not found in resources directory\n";
        return false;
    } else {
        echo "✅ Binary file $binary_name found in resources directory\n";
        return true;
    }
}

function ensure_signature_file_exists($binary_name)
{
    $signature_file = OUTPUT_DIR . "/" . $binary_name . ".sig";

    if (!file_exists($signature_file)) {
        echo "❌ Required signature file $binary_name.sig not found in resources directory\n";
        return false;
    } else {
        echo "✅ Signature file $binary_name.sig found in resources directory\n";
        return true;
    }
}

function ensure_public_key_file_exists()
{
    $public_key_file = OUTPUT_DIR . "/public.pem";
    if (!file_exists($public_key_file)) {
        echo "❌ Required public key file public.pem not found in resources directory\n";
        return false;
    } else {
        echo "✅ Public key file public.pem found in resources directory\n";
        return true;
    }
}


function ensure_ffi_enabled()
{
    $ffi_enable_value = ini_get('ffi.enable');
    $ffi_enabled = $ffi_enable_value === '1' || $ffi_enable_value === 'true';

    if (!$ffi_enabled) {
        echo "❌ 'ini.ffi.enable' is not enabled\n";
    } else {
        echo "✅ 'ini.ffi.enable' is enabled\n";
    }

    return $ffi_enabled;
}

function ffi_binary_verification_disabled()
{
    $ffi_binary_verification_disabled = ini_get('ini.ffi.statsig_binary_verification');
    return $ffi_binary_verification_disabled === '0' || $ffi_binary_verification_disabled === 'false';
}

function verify_binary($binary_name)
{
    echo "\n-- Verifying FFI Binary --\n";
    if (ffi_binary_verification_disabled()) {
        echo "✅ FFI binary verification is disabled, skipping verification\n";
        return true;
    }

    if (!extension_loaded('openssl')) {
        echo "✅ OpenSSL extension is not loaded, verification will be skipped\n" .
            "If you would like to verify the binary, please install the openssl extension.\n";
        return true;
    }
    // not hard failing verification for now
    if (!ensure_signature_file_exists($binary_name)) {
        echo "❌ Signature file not found, verification will be skipped\n";
        return true;
    }

    if (!ensure_public_key_file_exists()) {
        echo "❌ Public key file not found, verification will be skipped\n" .
            "If you would like to verify the binary, please check that downloading the public key from " . DOMAIN . " was successful.\n";
        return true;
    }

    $binary_path = OUTPUT_DIR . "/" . $binary_name;
    $signature_path = OUTPUT_DIR . "/" . $binary_name . ".sig";
    $public_key_path = OUTPUT_DIR . "/public.pem";

    $binary = file_get_contents($binary_path);
    $signature = file_get_contents($signature_path);
    $publicKey = file_get_contents($public_key_path);

    $pubKeyId = openssl_pkey_get_public($publicKey);
    if ($pubKeyId === false) {
        echo "❌ Failed to load public key, verification will be skipped\n";
        return true;
    }

    $ok = openssl_verify($binary, $signature, $pubKeyId, OPENSSL_ALGO_SHA256);
    if ($ok === 1) {
        echo "✅ FFI binary verification is successful\n";
    } else {
        echo "❌ FFI binary verification failed, the binary may be corrupted\n";
        return false;
    }

    return true;
}

$system_info = get_system_info();
ensure_bin_dir_exists();
remove_existing_statsig_resources();

$zip_file_path = download_binary($system_info);
unzip_binary($zip_file_path);
download_header();
download_public_key();


echo "\n-- Ensuring Resources Exist --\n";
$header_found = ensure_header_file_exists();
$binary_name = get_binary_name($system_info);
$binary_found = ensure_binary_file_exists($binary_name);
$ffi_enabled = ensure_ffi_enabled();
$verified = verify_binary($binary_name);
echo "-----------------------------------\n";

if (!$header_found || !$binary_found || !$ffi_enabled || !$verified) {
    exit(1);
}
