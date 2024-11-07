use std::{io::Write, path::PathBuf};

use crate::utils::*;
use colored::*;
extern crate glob;
use zip::write::SimpleFileOptions;

pub async fn execute(glob_pattern: &str, output_file: &str) {
    print_title("🗜️ ", "Zipping Files", Color::Yellow);

    let files = get_files(glob_pattern);
    println!("Found count:{}", files.len());

    let zip_file = std::fs::File::create(output_file).expect("Failed to create zip file");
    let mut zip = zip::ZipWriter::new(zip_file);

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    println!("Creating zip file: {}", output_file);

    for entry in &files {
        match entry {
            Ok(path) => {
                println!("Adding file: {}", path.display());

                let file_path = path.to_str().expect("Failed to convert path to string");
                let parts: Vec<&str> = file_path.split('/').collect();
                let storage_name = if parts.len() > 2 {
                    parts[parts.len() - 2..].join("/")
                } else {
                    file_path.to_string()
                };

                zip.start_file(storage_name, options)
                    .expect("Failed to start zip file entry");

                let file_contents = std::fs::read(path).expect("Failed to read file contents");

                zip.write_all(&file_contents)
                    .expect("Failed to write file contents to zip");
            }
            Err(e) => println!("Error processing file: {}", e),
        }
    }

    zip.finish().expect("Failed to finish writing zip file");

    println!("Zip file created: {}", output_file);
}

fn get_files(glob_pattern: &str) -> Vec<Result<PathBuf, glob::GlobError>> {
    let patterns = glob_pattern.split(",").collect::<Vec<_>>();

    let mut files: Vec<_> = Vec::new();

    for pattern in patterns {
        let globbed: Vec<Result<PathBuf, glob::GlobError>> =
            glob::glob(pattern).expect("Failed to find files").collect();
        files.extend(globbed);
    }

    files
}
