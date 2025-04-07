use serde::Deserialize;

use crate::{
    compression::zstd_decompression_dict::DictionaryDecoder, spec_types::SpecsResponse, StatsigErr,
};

#[derive(Deserialize)]
struct DictCompressedSpecsResponse {
    #[serde(rename = "s")]
    pub specs: Vec<u8>,
    #[serde(rename = "d_id")]
    pub dict_id: Option<String>,
    #[serde(rename = "d")]
    pub dict_buff: Option<Vec<u8>>,
}

impl DictCompressedSpecsResponse {
    fn decompress(
        self,
        cached_dict: Option<&DictionaryDecoder>,
    ) -> Result<DecodedSpecsResponse, StatsigErr> {
        let decompression_dict_to_use =
            select_decompression_dict_for_response(self.dict_id, self.dict_buff, cached_dict)?;

        match decompression_dict_to_use {
            None => {
                // Response was not compressed
                let parsed = serde_json::from_slice::<SpecsResponse>(&self.specs).map_err(|e| {
                    StatsigErr::JsonParseError("SpecsResponse".to_string(), e.to_string())
                })?;
                Ok(DecodedSpecsResponse {
                    specs: parsed,
                    decompression_dict: None,
                })
            }
            Some(dict) => {
                // Response was compressed, so we need to decompress first then parse
                let uncompressed = dict.decompress(&self.specs)?;
                let parsed =
                    serde_json::from_slice::<SpecsResponse>(&uncompressed).map_err(|e| {
                        StatsigErr::JsonParseError("SpecsResponse".to_string(), e.to_string())
                    })?;
                Ok(DecodedSpecsResponse {
                    specs: parsed,
                    decompression_dict: Some(dict),
                })
            }
        }
    }
}

fn select_decompression_dict_for_response(
    response_dict_id: Option<String>,
    response_dict_buff: Option<Vec<u8>>,
    cached_dict: Option<&DictionaryDecoder>,
) -> Result<Option<DictionaryDecoder>, StatsigErr> {
    match response_dict_id {
        None => Ok(None),
        Some(response_dict_id) => {
            if let Some(cached_dict) = cached_dict.filter(|d| d.get_dict_id() == response_dict_id) {
                return Ok(Some(cached_dict.clone()));
            }

            response_dict_buff
                .map(|dict_buff| DictionaryDecoder::new(None, response_dict_id.clone(), &dict_buff))
                .map(|dict| Ok(Some(dict)))
                .unwrap_or_else(|| {
                    Err(StatsigErr::ZstdDictCompressionError(format!(
                        "Cannot decompress response compressed with dict_id: {}, \
                                because the appropriate dictionary is not cached \
                                and the response does not contain one.",
                        response_dict_id
                    )))
                })
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CompressedSpecsResponse {
    DictCompressed(DictCompressedSpecsResponse),
    Uncompressed(SpecsResponse),
}

pub struct DecodedSpecsResponse {
    pub specs: SpecsResponse,
    pub decompression_dict: Option<DictionaryDecoder>,
}

impl DecodedSpecsResponse {
    pub fn from_str(
        response_str: &str,
        decompression_dict: Option<&DictionaryDecoder>,
    ) -> Result<DecodedSpecsResponse, StatsigErr> {
        serde_json::from_str::<CompressedSpecsResponse>(response_str)
            .map_err(|e| {
                StatsigErr::JsonParseError("CompressedSpecsResponse".to_string(), e.to_string())
            })
            .and_then(|compressed| Self::from_compressed(compressed, decompression_dict))
    }

    fn from_compressed(
        compressed: CompressedSpecsResponse,
        decompression_dict: Option<&DictionaryDecoder>,
    ) -> Result<DecodedSpecsResponse, StatsigErr> {
        match compressed {
            CompressedSpecsResponse::DictCompressed(compressed_response) => {
                eprintln!("Parsing dict compressed specs");
                compressed_response.decompress(decompression_dict)
            }
            CompressedSpecsResponse::Uncompressed(specs) => {
                eprintln!("Parsing uncompressed specs");
                Ok(DecodedSpecsResponse {
                    specs,
                    decompression_dict: decompression_dict.cloned(),
                })
            }
        }
    }
}
