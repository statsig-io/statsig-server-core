use statsig_rust::spec_types::*;
use statsig_rust::spec_types_encoded::*;

mod tests {
    use statsig_rust::compression::zstd_decompression_dict::DictionaryDecoder;

    use super::*;

    // Tests that we can parse a response into a DecodedSpecsResponse,
    // given that the response was not zstd dictionary compressed.
    #[test]
    fn decode_specs_response_spec_only_works() {
        let original_spec = serde_json::from_slice::<SpecsResponseFull>(
            &std::fs::read("./tests/data/shared_dict_original_dcs.json")
                .expect("Failed to read file"),
        )
        .expect("Failed to parse SpecsResponseFull");
        let spec_response_slice =
            std::fs::read("./tests/data/shared_dict_response_uncompressed.json")
                .expect("Failed to read file");

        let spec_response = DecodedSpecsResponse::from_slice(&spec_response_slice, None)
            .expect("Failed to parse SpecsResponse");

        assert!(spec_response.decompression_dict.is_none());
        let decoded_spec = match spec_response.specs {
            SpecsResponse::Full(full) => *full,
            _ => panic!("Expected Full response"),
        };
        assert_eq!(decoded_spec, original_spec);
    }

    // Tests that we can parse a response into a DecodedSpecsResponse,
    // given that the response WAS zstd dictionary compressed AND the decompression dictionary was included in the response.
    #[test]
    fn decode_specs_response_compressed_dict_in_response_works() {
        let original_spec = serde_json::from_slice::<SpecsResponseFull>(
            &std::fs::read("./tests/data/shared_dict_original_dcs.json")
                .expect("Failed to read file"),
        )
        .expect("Failed to parse SpecsResponseFull");
        let spec_response_slice = std::fs::read("./tests/data/shared_dict_response_with_dict.json")
            .expect("Failed to read file");

        let spec_response = DecodedSpecsResponse::from_slice(&spec_response_slice, None)
            .expect("Failed to parse SpecsResponse");

        assert!(spec_response.decompression_dict.is_some());
        let decoded_spec = match spec_response.specs {
            SpecsResponse::Full(full) => *full,
            _ => panic!("Expected Full response"),
        };
        assert_eq!(decoded_spec, original_spec);
    }

    // Tests that we can parse a response into a DecodedSpecsResponse,
    // given that the response WAS zstd dictionary compressed AND the decompression dictionary was included in the response,
    // but at the same time, we have the WRONG decompression dictionary cached.
    #[test]
    fn decode_specs_response_compressed_dict_in_response_wrong_dict_provided_works() {
        let original_spec = serde_json::from_slice::<SpecsResponseFull>(
            &std::fs::read("./tests/data/shared_dict_original_dcs.json")
                .expect("Failed to read file"),
        )
        .expect("Failed to parse SpecsResponseFull");
        let dict_buff = serde_json::from_slice::<Vec<u8>>(
            &std::fs::read("./tests/data/shared_dict_dict_only.json").expect("Failed to read file"),
        )
        .expect("Failed to parse dict");
        let spec_response_str = std::fs::read("./tests/data/shared_dict_response_with_dict.json")
            .expect("Failed to read file");

        let decoder_dict = DictionaryDecoder::new(None, "WRONG_DICT_ID".to_string(), &dict_buff);
        let spec_response =
            DecodedSpecsResponse::from_slice(&spec_response_str, Some(&decoder_dict))
                .expect("Failed to parse SpecsResponse");

        assert!(spec_response.decompression_dict.is_some());
        let decoded_spec = match spec_response.specs {
            SpecsResponse::Full(full) => *full,
            _ => panic!("Expected Full response"),
        };
        assert_eq!(decoded_spec, original_spec);
    }

    // Tests that we can parse a response into a DecodedSpecsResponse,
    // given that the response WAS zstd dictionary compressed AND the decompression dictionary was NOT included in the response,
    // but we had the CORRECT decompression dictionary cached.
    #[test]
    fn decode_specs_response_compressed_no_dict_in_response_works() {
        let original_spec = serde_json::from_slice::<SpecsResponseFull>(
            &std::fs::read("./tests/data/shared_dict_original_dcs.json")
                .expect("Failed to read file"),
        )
        .expect("Failed to parse SpecsResponseFull");
        let dict_buff = serde_json::from_slice::<Vec<u8>>(
            &std::fs::read("./tests/data/shared_dict_dict_only.json").expect("Failed to read file"),
        )
        .expect("Failed to parse dict");
        let spec_response_slice =
            std::fs::read("./tests/data/shared_dict_response_without_dict.json")
                .expect("Failed to read file");

        let decoder_dict = DictionaryDecoder::new(None, "ABC123".to_string(), &dict_buff);
        let spec_response =
            DecodedSpecsResponse::from_slice(&spec_response_slice, Some(&decoder_dict))
                .expect("Failed to parse SpecsResponse");

        assert!(spec_response.decompression_dict.is_some());
        let decoded_spec = match spec_response.specs {
            SpecsResponse::Full(full) => *full,
            _ => panic!("Expected Full response"),
        };
        assert_eq!(decoded_spec, original_spec);
    }

    // Tests that we throw an exception when we try to parse a response into a DecodedSpecsResponse,
    // given that the response WAS zstd dictionary compressed AND the decompression dictionary was NOT included in the response,
    // and we had the WRONG decompression dictionary cached.
    #[test]
    fn decode_specs_response_compressed_no_dict_in_response_wrong_dict_provided_throws() {
        let dict_buff = serde_json::from_slice::<Vec<u8>>(
            &std::fs::read("./tests/data/shared_dict_dict_only.json").expect("Failed to read file"),
        )
        .expect("Failed to parse dict");
        let spec_response_slice =
            std::fs::read("./tests/data/shared_dict_response_without_dict.json")
                .expect("Failed to read file");

        let decoder_dict = DictionaryDecoder::new(None, "WRONG_DICT_ID".to_string(), &dict_buff);
        let spec_response_result =
            DecodedSpecsResponse::from_slice(&spec_response_slice, Some(&decoder_dict));

        assert!(spec_response_result.is_err());
    }
}
