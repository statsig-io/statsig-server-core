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
                    return Err(new_parse_err(e));
                }
            }
        }

        Ok(self.buf.split_to(required_len))
    }

    fn read_length_delimiter(&mut self) -> Result<usize, StatsigErr> {
        let len_buf = &mut [0u8; 10];

        let read_len = self
            .brotli_decompressor
            .read(len_buf)
            .map_err(new_parse_err)?;

        if read_len > 0 {
            self.buf.extend_from_slice(&len_buf[..read_len]);
        }

        let data_len = prost::decode_length_delimiter(self.buf.as_ref()).map_err(new_parse_err)?;
        let required_len = prost::length_delimiter_len(data_len) + data_len;

        Ok(required_len)
    }
}

fn new_parse_err<E>(err_string: E) -> StatsigErr
where
    E: std::fmt::Display,
{
    StatsigErr::ProtobufParseError("BrotliDecompressor".to_string(), err_string.to_string())
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
