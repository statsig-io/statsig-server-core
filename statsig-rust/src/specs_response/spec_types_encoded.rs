use crate::{compression::zstd_decompression_dict::DictionaryDecoder, StatsigErr};
use serde::Deserialize;

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
        place: &mut TSpecs,
        cached_dict: Option<&DictionaryDecoder>,
    ) -> Result<Option<DictionaryDecoder>, StatsigErr>
    where
        TSpecs: for<'de> Deserialize<'de>,
    {
        let decompression_dict =
            select_decompression_dict_for_response(self.dict_id, self.dict_buff, cached_dict)?;

        let mut bytes = self.specs;
        if let Some(dict) = &decompression_dict {
            bytes = dict.decompress(&bytes)?;
        }

        deserialize_response_in_place(&bytes, place)?;

        Ok(decompression_dict)
    }
}

pub struct DecodedSpecsResponse;

impl DecodedSpecsResponse {
    pub fn from_slice<TSpecs>(
        response_slice: &[u8],
        place: &mut TSpecs,
        decompression_dict: Option<&DictionaryDecoder>,
    ) -> Result<Option<DictionaryDecoder>, StatsigErr>
    where
        TSpecs: for<'de> Deserialize<'de>,
    {
        let compressed = serde_json::from_slice::<DictCompressedSpecsResponse>(response_slice);
        if let Ok(compressed) = compressed {
            return compressed.decompress::<TSpecs>(place, decompression_dict);
        }

        deserialize_response_in_place(response_slice, place)?;

        Ok(decompression_dict.cloned())
    }
}

fn deserialize_response_in_place<TSpecs>(
    response_slice: &[u8],
    place: &mut TSpecs,
) -> Result<(), StatsigErr>
where
    TSpecs: for<'de> Deserialize<'de>,
{
    let mut deserializer = serde_json::Deserializer::from_slice(response_slice);
    TSpecs::deserialize_in_place(&mut deserializer, place)
        .map_err(|e| StatsigErr::JsonParseError("SpecsResponse".to_string(), e.to_string()))?;

    Ok(())
}

fn select_decompression_dict_for_response(
    response_dict_id: Option<String>,
    response_dict_buff: Option<Vec<u8>>,
    cached_dict: Option<&DictionaryDecoder>,
) -> Result<Option<DictionaryDecoder>, StatsigErr> {
    let response_dict_id = match response_dict_id {
        Some(id) => id,
        None => return Ok(None),
    };

    if let Some(cached_dict) = cached_dict.filter(|d| d.get_dict_id() == response_dict_id) {
        return Ok(Some(cached_dict.clone()));
    }

    if let Some(dict_buff) = response_dict_buff {
        return Ok(Some(DictionaryDecoder::new(
            None,
            response_dict_id.clone(),
            &dict_buff,
        )));
    }

    Err(StatsigErr::ZstdDictCompressionError(format!(
        "Cannot decompress response compressed with dict_id: {response_dict_id}, \
                        because the appropriate dictionary is not cached \
                        and the response does not contain one."
    )))
}
