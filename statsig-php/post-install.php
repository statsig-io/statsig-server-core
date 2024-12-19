<?php

const OUTPUT_DIR = "resources";
const VERSION = "0.0.1-beta.2";

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

function download_binary($system_info)
{
    $binary_map = [
        "macos-x86_64" => "statsig-ffi-x86_64-apple-darwin.zip",
        "macos-aarch64" => "statsig-ffi-aarch64-apple-darwin.zip",
        "linux-aarch64" => "statsig-ffi-aarch64-unknown-linux-gnu.zip",
        "linux-x86_64" => "statsig-ffi-x86_64-unknown-linux-gnu.zip",
        "windows-x86_64" => "statsig-ffi-x86_64-pc-windows-msvc.zip",
        "windows-aarch64" => "statsig-ffi-aarch64-pc-windows-msvc.zip",
    ];

    $binary_file = $binary_map[$system_info[0] . "-" . $system_info[1]] ?? null;

    if ($binary_file === null) {
        echo "No binary found for: {$system_info[0]} {$system_info[1]}\n";
        exit(1);
    }

    $url = "https://github.com/statsig-io/statsig-core-php/releases/download/" . VERSION . "/" . $binary_file;

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
    $url = "https://github.com/statsig-io/statsig-core-php/releases/download/" . VERSION . "/statsig_ffi.h";
    echo "Downloading header from $url\n";

    $output_path = OUTPUT_DIR . "/statsig_ffi.h";
    file_put_contents($output_path, file_get_contents($url));
}


$system_info = get_system_info();
ensure_bin_dir_exists();
$zip_file_path = download_binary($system_info);
unzip_binary($zip_file_path);
download_header();
