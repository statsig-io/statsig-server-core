use statsig_rust::compression::zstd_decompression_dict::DictionaryDecoder;

mod tests {
    use super::*;
    #[test]
    fn can_create_decoder() {
        // Arrange
        let dict_id = "test_dict".to_string();
        let dict_buff = "Hello".as_bytes();

        // Act, Assert
        DictionaryDecoder::new(None, dict_id, dict_buff);
    }

    #[test]
    fn decompressed_matches_original() {
        // Arrange
        let dict_id = "test_dict";
        let dict_buff = "Hello".as_bytes();
        let cdict = zstd::dict::EncoderDictionary::copy(dict_buff, 3);
        let mut encoder = zstd::stream::Encoder::with_prepared_dictionary(
            std::io::Cursor::new(Vec::new()),
            &cdict,
        )
        .expect("Failed to create encoder");

        // Act
        let original = "Hello, world!".as_bytes();
        let mut original_reader = std::io::Cursor::new(original);
        std::io::copy(&mut original_reader, &mut encoder).expect("Failed to compress.");
        let compressed = encoder
            .finish()
            .expect("Failed to finish compression.")
            .into_inner();

        let decoder = DictionaryDecoder::new(None, dict_id.to_string(), dict_buff);
        let decompressed = decoder
            .decompress(&compressed)
            .expect("Failed to decompress.");

        // Assert
        assert_eq!(original, decompressed);
    }

    #[test]
    fn can_reuse_decoder() {
        // Arrange
        let dict_id = "test_dict";
        let dict_buff = "Hello".as_bytes();
        let cdict = zstd::dict::EncoderDictionary::copy(dict_buff, 3);
        let mut encoder = zstd::stream::Encoder::with_prepared_dictionary(
            std::io::Cursor::new(Vec::new()),
            &cdict,
        )
        .expect("Failed to create encoder");

        // Act
        let original = "Hello, world!".as_bytes();
        let mut original_reader = std::io::Cursor::new(original);
        std::io::copy(&mut original_reader, &mut encoder).expect("Failed to compress.");
        let compressed = encoder
            .finish()
            .expect("Failed to finish compression.")
            .into_inner();

        let decoder = DictionaryDecoder::new(None, dict_id.to_string(), dict_buff);
        let decompressed = decoder
            .decompress(&compressed)
            .expect("Failed to decompress.");
        let decompressed_2 = decoder
            .decompress(&compressed)
            .expect("Failed to decompress a second time.");

        // Assert
        assert_eq!(original, decompressed);
        assert_eq!(original, decompressed_2);
    }
}
