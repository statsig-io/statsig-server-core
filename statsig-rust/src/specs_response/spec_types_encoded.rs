use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::{compression::zstd_decompression_dict::DictionaryDecoder, StatsigErr};
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
    fn decompress<TSpecs>(
        self,
        cached_dict: Option<&DictionaryDecoder>,
    ) -> Result<DecodedSpecsResponse<TSpecs>, StatsigErr>
    where
        TSpecs: for<'de> Deserialize<'de>,
    {
        let decompression_dict_to_use =
            select_decompression_dict_for_response(self.dict_id, self.dict_buff, cached_dict)?;

        match decompression_dict_to_use {
            None => {
                // Response was not compressed
                let parsed = serde_json::from_slice::<TSpecs>(&self.specs).map_err(|e| {
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
                let parsed = serde_json::from_slice::<TSpecs>(&uncompressed).map_err(|e| {
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
#[serde(untagged, bound = "TSpecs: DeserializeOwned")]
enum CompressedSpecsResponse<TSpecs> {
    DictCompressed(DictCompressedSpecsResponse),
    Uncompressed(TSpecs),
}

pub struct DecodedSpecsResponse<TSpecs> {
    pub specs: TSpecs,
    pub decompression_dict: Option<DictionaryDecoder>,
}

impl<TSpecs> DecodedSpecsResponse<TSpecs> {
    pub fn from_slice(
        response_slice: &[u8],
        decompression_dict: Option<&DictionaryDecoder>,
    ) -> Result<DecodedSpecsResponse<TSpecs>, StatsigErr>
    where
        TSpecs: for<'de> Deserialize<'de>,
    {
        serde_json::from_slice::<CompressedSpecsResponse<TSpecs>>(response_slice)
            .map_err(|e| {
                StatsigErr::JsonParseError("CompressedSpecsResponse".to_string(), e.to_string())
            })
            .and_then(|compressed| Self::from_compressed(compressed, decompression_dict))
    }

    fn from_compressed(
        compressed: CompressedSpecsResponse<TSpecs>,
        decompression_dict: Option<&DictionaryDecoder>,
    ) -> Result<DecodedSpecsResponse<TSpecs>, StatsigErr>
    where
        TSpecs: for<'de> Deserialize<'de>,
    {
        match compressed {
            CompressedSpecsResponse::DictCompressed(compressed_response) => {
                compressed_response.decompress(decompression_dict)
            }
            CompressedSpecsResponse::Uncompressed(specs) => Ok(DecodedSpecsResponse {
                specs,
                decompression_dict: decompression_dict.cloned(),
            }),
        }
    }
}
