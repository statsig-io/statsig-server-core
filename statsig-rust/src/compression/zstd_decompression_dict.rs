use std::sync::Arc;

use crate::observability::ops_stats::OpsStatsForInstance;
use crate::observability::sdk_errors_observer::ErrorBoundaryEvent;
use serde::Serialize;
use std::fmt::Debug;

use crate::{log_error_to_statsig_and_console, StatsigErr};

const TAG: &str = stringify!(DictionaryDecoder);

/// Wraps zstd::dict::DecoderDictionary.
/// No need to wrap this in Arc; its internals are already wrapped in Arc.
#[derive(Clone)]
pub struct DictionaryDecoder {
    inner: Arc<DictionaryDecoderInner>,
}

impl Serialize for DictionaryDecoder {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl Debug for DictionaryDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DictionaryDecoder {{ dict_id: {:?} }}",
            self.inner.dict_id
        )
    }
}

#[derive(Serialize)]
struct DictionaryDecoderInner {
    #[serde(skip)]
    ops_stats: Option<Arc<OpsStatsForInstance>>,
    dict_id: String,
    #[serde(skip)]
    ddict: zstd::dict::DecoderDictionary<'static>,
}

impl DictionaryDecoder {
    pub fn new(
        ops_stats: Option<Arc<OpsStatsForInstance>>,
        dict_id: String,
        dict_buff: &[u8],
    ) -> Self {
        let ddict = zstd::dict::DecoderDictionary::copy(dict_buff);
        Self {
            inner: Arc::new(DictionaryDecoderInner {
                ops_stats,
                dict_id,
                ddict,
            }),
        }
    }

    pub fn get_dict_id(&self) -> &str {
        &self.inner.dict_id
    }

    pub fn decompress(&self, compressed: &[u8]) -> Result<Vec<u8>, StatsigErr> {
        let mut decompressed = std::io::Cursor::new(Vec::new());
        let compressed_reader = std::io::Cursor::new(compressed);
        let mut decoder =
            zstd::stream::Decoder::with_prepared_dictionary(compressed_reader, &self.inner.ddict)
                .map_err(|e| {
                if let Some(ops_stats) = &self.inner.ops_stats {
                    log_error_to_statsig_and_console!(
                        ops_stats,
                        TAG,
                        "Unexpected error while constructing decoder with dictionary {}: {}",
                        self.inner.dict_id,
                        e
                    );
                }
                StatsigErr::ZstdError(format!(
                    "Unexpected error while constructing decoder with dictionary {}: {}",
                    self.inner.dict_id, e
                ))
            })?;
        std::io::copy(&mut decoder, &mut decompressed).map_err(|e| {
            StatsigErr::ZstdError(format!("Unexpected error while decompressing data: {}", e))
        })?;

        Ok(decompressed.into_inner())
    }
}
