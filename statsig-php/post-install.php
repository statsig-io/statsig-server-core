<?php

const OUTPUT_DIR = "resources";
const VERSION = "0.0.10-beta.2";

if (getenv('SKIP_STATSIG_POST_INSTALL') === 'true') {
    exit(0);
}

function get_system_info()
{
    $os = PHP_OS_FAMILY;
    $arch = php_uname('m');

    if ($arch === 'amd64' || $arch === 'x86_64') {
        $arch = 'x86_64';
    } else if ($arch === 'arm64' || $arch === 'aarch64') {
        $arch = 'aarch64';
    } else {
        echo "Unsupported architecture: $arch\n";
        exit(1);
    }

    if ($os === 'Darwin') {
        $os = 'macos';
    } else if ($os === 'Linux') {
        $os = 'linux';
    } else if ($os === 'Windows') {
        $os = 'windows';
    } else {
        echo "Unsupported OS: $os\n";
        exit(1);
    }

    echo "System Info: {$os} {$arch}\n";

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
    ];

    $system_tag = $system_info[0] . "-" . $system_info[1];
    if (isMusl($system_info[0])) {
        $system_tag .= "-musl";
    }

    $binary_file = $binary_map[$system_tag] ?? null;

    if ($binary_file === null) {
        echo "No binary found for: {$system_tag}\n";
        exit(1);
    }

    $url = "https://github.com/statsig-io/statsig-server-core/releases/download/" . VERSION . "/" . $binary_file;

    echo "Downloading binary from $url\n";

    $output_path = OUTPUT_DIR . "/" . $binary_file;

    file_put_contents($output_path, file_get_contents($url));

    return $output_path;
}

function unzip_binary($zip_file_path)
{
    echo "Unzipping binary\n";
    $zip = new ZipArchive();
    if ($zip->open($zip_file_path) === TRUE) {
        for ($i = 0; $i < $zip->numFiles; $i++) {
            $filename = $zip->getNameIndex($i);
            if (in_array($filename, ['libstatsig_ffi.dylib', 'statsig_ffi.dll', 'libstatsig_ffi.so'])) {
                $zip->extractTo(OUTPUT_DIR, $filename);
                echo "Extracted $filename\n";
            }
        }
        $zip->close();
        echo "Binary unzipped to " . OUTPUT_DIR . "\n";
    } else {
        echo "Failed to open zip file\n";
        exit(1);
    }

    echo "Binary unzipped to " . OUTPUT_DIR . "\n";

    echo "Deleting zip file\n";
    if (unlink($zip_file_path)) {
        echo "Successfully deleted $zip_file_path\n";
    } else {
        echo "Failed to delete $zip_file_path\n";
    }
}

function download_header()
{
    $url = "https://github.com/statsig-io/statsig-server-core/releases/download/" . VERSION . "/statsig_ffi.h";

    echo "Downloading header from $url\n";

    $output_path = OUTPUT_DIR . "/statsig_ffi.h";
    file_put_contents($output_path, file_get_contents($url));
}


$system_info = get_system_info();
ensure_bin_dir_exists();
remove_existing_statsig_resources();

$zip_file_path = download_binary($system_info);
unzip_binary($zip_file_path);
download_header();
