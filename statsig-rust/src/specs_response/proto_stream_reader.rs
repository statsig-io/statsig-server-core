use std::io::Read;

use crate::{
    networking::{ResponseData, ResponseDataStream},
    StatsigErr,
};
use brotli::Decompressor;
use bytes::BytesMut;

pub const BUFFER_SIZE: usize = 4096;

pub struct ProtoStreamReader<'a> {
    brotli_decompressor: Decompressor<StreamBorrower<'a>>,

    scratch: [u8; BUFFER_SIZE],
    buf: BytesMut,
}

impl<'a> ProtoStreamReader<'a> {
    pub fn new(data: &'a mut ResponseData) -> Self {
        let stream_borrower = StreamBorrower::new(data);
        let brotli_decompressor = Decompressor::new(stream_borrower, BUFFER_SIZE);

        Self {
            brotli_decompressor,
            scratch: [0u8; BUFFER_SIZE],
            buf: BytesMut::new(),
        }
    }

    pub fn read_next_delimited_proto(&mut self) -> Result<BytesMut, StatsigErr> {
        let required_len = self.read_length_delimiter()?;

        while self.buf.len() < required_len {
            match self.brotli_decompressor.read(&mut self.scratch) {
                Ok(0) => {
                    return Ok(self.buf.split_to(required_len));
                }
                Ok(n) => {
                    self.buf.extend_from_slice(&self.scratch[..n]);
                }
                Err(e) => {
                    return Err(StatsigErr::ProtobufParseError(
                        "BrotliDecompressorRead".to_string(),
                        e.to_string(),
                    ));
                }
            }
        }

        Ok(self.buf.split_to(required_len))
    }

    pub fn sample_current_buf(&self) -> String {
        let len = std::cmp::min(self.buf.len(), 100);
        let slice = &self.buf.as_ref()[..len];
        String::from_utf8(slice.to_vec()).unwrap_or_default()
    }

    fn read_length_delimiter(&mut self) -> Result<usize, StatsigErr> {
        loop {
            match prost::decode_length_delimiter(self.buf.as_ref()) {
                Ok(data_len) => {
                    return Ok(prost::length_delimiter_len(data_len) + data_len);
                }
                Err(e) if self.buf.len() >= 10 => {
                    return Err(StatsigErr::ProtobufParseError(
                        "DecodeLengthDelimiter".to_string(),
                        e.to_string(),
                    ));
                }
                Err(_) => {
                    let read_len =
                        self.brotli_decompressor
                            .read(&mut self.scratch)
                            .map_err(|e| {
                                StatsigErr::ProtobufParseError(
                                    "ReadLengthDelimiter".to_string(),
                                    e.to_string(),
                                )
                            })?;

                    if read_len == 0 {
                        return Err(StatsigErr::ProtobufParseError(
                            "ReadLengthDelimiter".to_string(),
                            "unexpected EOF while reading length delimiter".to_string(),
                        ));
                    }

                    self.buf.extend_from_slice(&self.scratch[..read_len]);
                }
            }
        }
    }
}

struct StreamBorrower<'a> {
    stream: &'a mut dyn ResponseDataStream,
}

impl<'a> StreamBorrower<'a> {
    pub fn new(data: &'a mut ResponseData) -> Self {
        Self {
            stream: data.get_stream_mut(),
        }
    }
}

impl<'a> std::io::Read for StreamBorrower<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf)
    }
}
