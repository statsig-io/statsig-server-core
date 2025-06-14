<?php

const OUTPUT_DIR = "resources";
const VERSION = "0.5.2-beta.2506140235";

if (getenv('SKIP_STATSIG_POST_INSTALL') === 'true') {
    exit(0);
}

function get_amazon_linux_os()
{
    $osFile = '/etc/os-release';
    if (!file_exists($osFile)) {
        return null;
    }

    $osRelease = file_get_contents($osFile);
    if ($osRelease === false) {
        return null;
    }

    if (strpos($osRelease, 'Amazon Linux 2023') !== false) {
        return 'amazonlinux2023';
    }

    if (strpos($osRelease, 'Amazon Linux 2') !== false) {
        return 'amazonlinux2';
    }

    return null;
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

    $os = get_amazon_linux_os();
    if ($os === null) {
        $os = get_os();
    }

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
        "macos-aarch64" => "statsig-ffi-" . VERSION . "-aarch64-apple-darwin-shared.zip",
        "macos-x86_64" => "statsig-ffi-" . VERSION . "-x86_64-apple-darwin-shared.zip",

        "linux-aarch64" => "statsig-ffi-" . VERSION . "-debian-aarch64-unknown-linux-gnu-shared.zip",
        "linux-x86_64" => "statsig-ffi-" . VERSION . "-debian-x86_64-unknown-linux-gnu-shared.zip",

        "linux-aarch64-musl" => "statsig-ffi-" . VERSION . "-alpine-aarch64-unknown-linux-musl-shared.zip",
        "linux-x86_64-musl" => "statsig-ffi-" . VERSION . "-alpine-x86_64-unknown-linux-musl-shared.zip",

        "amazonlinux2023-aarch64" => "statsig-ffi-" . VERSION . "-amazonlinux2023-aarch64-unknown-linux-gnu-shared.zip",
        "amazonlinux2023-x86_64" => "statsig-ffi-" . VERSION . "-amazonlinux2023-x86_64-unknown-linux-gnu-shared.zip",

        "amazonlinux2-aarch64" => "statsig-ffi-" . VERSION . "-amazonlinux2-aarch64-unknown-linux-gnu-shared.zip",
        "amazonlinux2-x86_64" => "statsig-ffi-" . VERSION . "-amazonlinux2-x86_64-unknown-linux-gnu-shared.zip",
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


function ensure_header_file_exists()
{
    $dir = OUTPUT_DIR;
    $header_file = $dir . "/statsig_ffi.h";

    if (!file_exists($header_file)) {
        echo "❌ Required header file statsig_ffi.h not found in resources directory\n";
        return false;
    } else {
        echo "✅ Header file statsig_ffi.h found in resources directory\n";
        return true;
    }
}

function ensure_binary_file_exists($system_info)
{
    $binary_name = "libstatsig_ffi.so";
    if ($system_info[0] === "macos") {
        $binary_name = "libstatsig_ffi.dylib";
    } else if ($system_info[0] === "windows") {
        $binary_name = "statsig_ffi.dll";
    }

    $dir = OUTPUT_DIR;
    $binary_file = $dir . "/" . $binary_name;

    if (!file_exists($binary_file)) {
        echo "❌ Required binary file $binary_name not found in resources directory\n";
        return false;
    } else {
        echo "✅ Binary file $binary_name found in resources directory\n";
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

$system_info = get_system_info();
ensure_bin_dir_exists();
remove_existing_statsig_resources();

$zip_file_path = download_binary($system_info);
unzip_binary($zip_file_path);
download_header();


echo "\n-- Ensuring Resources Exist --\n";
$header_found = ensure_header_file_exists();
$binary_found = ensure_binary_file_exists($system_info);
$ffi_enabled = ensure_ffi_enabled();
echo "-----------------------------------\n";

if (!$header_found || !$binary_found || !$ffi_enabled) {
    exit(1);
}
