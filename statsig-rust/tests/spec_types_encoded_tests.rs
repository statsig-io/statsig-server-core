use statsig_rust::specs_response::spec_types::*;
use statsig_rust::specs_response::spec_types_encoded::*;

lazy_static::lazy_static! {
    static ref SHARED_DICT_ORIGINAL_DCS: Vec<u8> = std::fs::read("./tests/data/shared_dict_original_dcs.json")
        .expect("Failed to read file");
    static ref SHARED_DICT_DICT_ONLY: Vec<u8> = std::fs::read("./tests/data/shared_dict_dict_only.json")
        .expect("Failed to read file");
    static ref SHARED_DICT_RESPONSE_UNCOMPRESSED: Vec<u8> = std::fs::read("./tests/data/shared_dict_response_uncompressed.json")
        .expect("Failed to read file");
    static ref SHARED_DICT_RESPONSE_WITH_DICT: Vec<u8> = std::fs::read("./tests/data/shared_dict_response_with_dict.json")
        .expect("Failed to read file");
    static ref SHARED_DICT_RESPONSE_WITHOUT_DICT: Vec<u8> = std::fs::read("./tests/data/shared_dict_response_without_dict.json")
        .expect("Failed to read file");
}

mod tests {
    use statsig_rust::compression::zstd_decompression_dict::DictionaryDecoder;

    use super::*;

    // Tests that we can parse a response into a DecodedSpecsResponse,
    // given that the response was not zstd dictionary compressed.
    #[test]
    fn decode_specs_response_spec_only_works() {
        let original_spec = serde_json::from_slice(&SHARED_DICT_ORIGINAL_DCS).unwrap();
        let mut place = SpecsResponseFull::default();
        let result =
            DecodedSpecsResponse::from_slice(&SHARED_DICT_RESPONSE_UNCOMPRESSED, &mut place, None)
                .unwrap();

        assert!(result.is_none());
        assert_eq!(place, original_spec);
    }

    // Tests that we can parse a response into a DecodedSpecsResponse,
    // given that the response WAS zstd dictionary compressed AND the decompression dictionary was included in the response.
    #[test]
    fn decode_specs_response_compressed_dict_in_response_works() {
        let original_spec = serde_json::from_slice(&SHARED_DICT_ORIGINAL_DCS).unwrap();
        let mut place = SpecsResponseFull::default();
        let result =
            DecodedSpecsResponse::from_slice(&SHARED_DICT_RESPONSE_WITH_DICT, &mut place, None)
                .unwrap();

        assert!(result.is_some());
        assert_eq!(place, original_spec);
    }

    // Tests that we can parse a response into a DecodedSpecsResponse,
    // given that the response WAS zstd dictionary compressed AND the decompression dictionary was included in the response,
    // but at the same time, we have the WRONG decompression dictionary cached.
    #[test]
    fn decode_specs_response_compressed_dict_in_response_wrong_dict_provided_works() {
        let original_spec = serde_json::from_slice(&SHARED_DICT_ORIGINAL_DCS).unwrap();
        let dict_buff = SHARED_DICT_DICT_ONLY.clone();
        let spec_response_slice = SHARED_DICT_RESPONSE_WITH_DICT.clone();

        let decoder_dict = DictionaryDecoder::new(None, "WRONG_DICT_ID".to_string(), &dict_buff);
        let mut place = SpecsResponseFull::default();
        let result =
            DecodedSpecsResponse::from_slice(&spec_response_slice, &mut place, Some(&decoder_dict))
                .unwrap();

        assert!(result.is_some());
        assert_eq!(place, original_spec);
    }

    // Tests that we can parse a response into a DecodedSpecsResponse,
    // given that the response WAS zstd dictionary compressed AND the decompression dictionary was NOT included in the response,
    // but we had the CORRECT decompression dictionary cached.
    #[test]
    fn decode_specs_response_compressed_no_dict_in_response_works() {
        let original_spec = serde_json::from_slice(&SHARED_DICT_ORIGINAL_DCS).unwrap();

        let dict_buff = serde_json::from_slice::<Vec<u8>>(&SHARED_DICT_DICT_ONLY.clone()).unwrap();
        let spec_response_slice = SHARED_DICT_RESPONSE_WITHOUT_DICT.clone();

        let decoder_dict = DictionaryDecoder::new(None, "ABC123".to_string(), &dict_buff);
        let mut place = SpecsResponseFull::default();
        let result =
            DecodedSpecsResponse::from_slice(&spec_response_slice, &mut place, Some(&decoder_dict))
                .unwrap();

        assert!(result.is_some());
        assert_eq!(place, original_spec);
    }

    // Tests that we throw an exception when we try to parse a response into a DecodedSpecsResponse,
    // given that the response WAS zstd dictionary compressed AND the decompression dictionary was NOT included in the response,
    // and we had the WRONG decompression dictionary cached.
    #[test]
    fn decode_specs_response_compressed_no_dict_in_response_wrong_dict_provided_throws() {
        let dict_buff = SHARED_DICT_DICT_ONLY.clone();
        let spec_response_slice = SHARED_DICT_RESPONSE_WITHOUT_DICT.clone();

        let decoder_dict = DictionaryDecoder::new(None, "WRONG_DICT_ID".to_string(), &dict_buff);
        let mut place = SpecsResponseFull::default();
        let result =
            DecodedSpecsResponse::from_slice(&spec_response_slice, &mut place, Some(&decoder_dict));

        assert!(result.is_err());
    }
}
