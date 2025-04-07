use crate::StatsigErr;
#[cfg(not(feature = "with_zstd"))]
use std::io::Write;

#[cfg(feature = "with_zstd")]
pub fn get_compression_format() -> String {
    "zstd".to_string()
}

#[cfg(not(feature = "with_zstd"))]
pub fn get_compression_format() -> String {
    "gzip".to_string()
}

#[cfg(feature = "with_zstd")]
pub fn compress_data(data: &[u8]) -> Result<Vec<u8>, StatsigErr> {
    zstd::bulk::compress(data, 12).map_err(|e| StatsigErr::ZstdError(e.to_string()))
}

#[cfg(not(feature = "with_zstd"))]
pub fn compress_data(data: &[u8]) -> Result<Vec<u8>, StatsigErr> {
    let mut compressed = Vec::new();
    let mut encoder = flate2::write::GzEncoder::new(&mut compressed, flate2::Compression::best());
    encoder
        .write_all(data)
        .map_err(|e| StatsigErr::GzipError(e.to_string()))?;
    encoder
        .finish()
        .map_err(|e| StatsigErr::GzipError(e.to_string()))?;
    Ok(compressed)
}
