use std::io::Write;

use crate::{
    networking::ResponseData,
    specs_response::proto_stream_reader::{self, ProtoStreamReader},
};
use brotli::enc::BrotliEncoderParams;
use prost::Message;

#[test]
fn test_reading_empty_data() {
    let mut data = ResponseData::from_bytes(b"".to_vec());
    let mut reader = ProtoStreamReader::new(&mut data);

    let result = reader.read_next_delimited_proto();
    assert!(result.is_err());
}

#[test]
fn test_reading_json_data() {
    let mut data =
        ResponseData::from_bytes(r#"{"has_updates":true}"#.to_string().as_bytes().to_vec());
    let mut reader = ProtoStreamReader::new(&mut data);

    let result = reader.read_next_delimited_proto();
    assert!(result.is_err());
}

// ------------------------------------------------------------ [ Proto Specs Bytes ]

const PROTO_SPECS_BYTES: &[u8] = include_bytes!("proto_specs.br");

#[test]
fn test_reading_proto_data() {
    let mut data = ResponseData::from_bytes(PROTO_SPECS_BYTES.to_vec());
    let mut reader = ProtoStreamReader::new(&mut data);

    let result = reader.read_next_delimited_proto();
    assert!(result.is_ok_and(|r| !r.is_empty()));
}

#[test]
fn test_proto_data_reads_until_done() {
    let mut data = ResponseData::from_bytes(PROTO_SPECS_BYTES.to_vec());
    let mut reader = ProtoStreamReader::new(&mut data);

    let mut last_result = None;
    while let Ok(result) = reader.read_next_delimited_proto() {
        last_result = Some(result);
    }

    let last_result = match last_result {
        Some(result) => String::from_utf8(result.to_vec()).unwrap(),
        None => panic!("No result was read"),
    };

    assert!(last_result.contains("DONE"));
}

#[test]
fn test_proto_missing_data() {
    let mut bytes = PROTO_SPECS_BYTES.to_vec();
    bytes.truncate(bytes.len() - 100);

    let mut data = ResponseData::from_bytes(bytes);
    let mut reader = ProtoStreamReader::new(&mut data);

    let mut error_result = None;

    for _ in 0..9999 {
        let result = reader.read_next_delimited_proto();
        if result.is_err() {
            error_result = Some(result);
            break;
        }
    }

    assert!(error_result.is_some_and(|r| r.is_err()));
}

// ------------------------------------------------------------ [ Dummy Protos ]

#[derive(Clone, prost::Message)]
struct TestMessage {
    #[prost(string, tag = "1")]
    pub content: String,
}

fn create_test_proto_data(messages: Vec<&str>) -> Vec<u8> {
    let mut encoded = Vec::new();

    // Encode each message as length-delimited
    for msg_content in messages {
        let msg = TestMessage {
            content: msg_content.to_string(),
        };
        let msg_bytes = msg.encode_to_vec();

        // Add length delimiter
        prost::encode_length_delimiter(msg_bytes.len(), &mut encoded).unwrap();
        encoded.extend_from_slice(&msg_bytes);
    }

    // Compress with brotli
    let mut compressed = Vec::new();
    let params = BrotliEncoderParams::default();
    {
        let mut writer = brotli::CompressorWriter::with_params(
            &mut compressed,
            proto_stream_reader::BUFFER_SIZE,
            &params,
        );

        writer.write_all(&encoded).unwrap();
        writer.flush().unwrap();
    }

    compressed
}

#[test]
fn test_multiple_consecutive_messages() {
    let test_data = create_test_proto_data(vec!["msg1", "msg2", "msg3"]);
    let mut data = ResponseData::from_bytes(test_data);
    let mut reader = ProtoStreamReader::new(&mut data);

    let msg1 = reader.read_next_delimited_proto().unwrap();
    let msg2 = reader.read_next_delimited_proto().unwrap();
    let msg3 = reader.read_next_delimited_proto().unwrap();

    let decoded1 = TestMessage::decode_length_delimited(msg1.as_ref()).unwrap();
    let decoded2 = TestMessage::decode_length_delimited(msg2.as_ref()).unwrap();
    let decoded3 = TestMessage::decode_length_delimited(msg3.as_ref()).unwrap();

    assert_eq!(decoded1.content, "msg1");
    assert_eq!(decoded2.content, "msg2");
    assert_eq!(decoded3.content, "msg3");
}

#[test]
fn test_zero_length_message() {
    let test_data = create_test_proto_data(vec![""]);

    let mut data = ResponseData::from_bytes(test_data);
    let mut reader = ProtoStreamReader::new(&mut data);

    let result = reader.read_next_delimited_proto().unwrap();
    let decoded = TestMessage::decode_length_delimited(result.as_ref()).unwrap();
    assert_eq!(decoded.content, "");
}

#[test]
fn test_message_at_buffer_boundary() {
    // Create messages that skirt the buffer boundary

    let less_than_buffer_size = "x".repeat(proto_stream_reader::BUFFER_SIZE - 1);
    let exact_size = "x".repeat(proto_stream_reader::BUFFER_SIZE);
    let more_than_buffer_size = "x".repeat(proto_stream_reader::BUFFER_SIZE + 1);
    let test_data = create_test_proto_data(vec![
        &less_than_buffer_size,
        &exact_size,
        &more_than_buffer_size,
    ]);
    let mut data = ResponseData::from_bytes(test_data);
    let mut reader = ProtoStreamReader::new(&mut data);

    for i in 0..2 {
        let result = reader.read_next_delimited_proto().unwrap();
        let decoded = TestMessage::decode_length_delimited(result.as_ref()).unwrap();

        match i {
            0 => assert_eq!(decoded.content.len(), proto_stream_reader::BUFFER_SIZE - 1),
            1 => assert_eq!(decoded.content.len(), proto_stream_reader::BUFFER_SIZE),
            2 => assert_eq!(decoded.content.len(), proto_stream_reader::BUFFER_SIZE + 1),
            _ => panic!("Invalid index"),
        }
    }
}

#[test]
fn test_calling_after_exhaustion() {
    let test_data = create_test_proto_data(vec!["single"]);
    let mut data = ResponseData::from_bytes(test_data);
    let mut reader = ProtoStreamReader::new(&mut data);

    // Read the one message
    let msg1 = reader.read_next_delimited_proto().unwrap();
    assert_eq!(
        TestMessage::decode_length_delimited(msg1.as_ref())
            .unwrap()
            .content,
        "single"
    );

    // Try to read again - should error
    let result = reader.read_next_delimited_proto();
    assert!(result.is_err());
}

#[test]
fn test_mixed_size_messages() {
    let test_data =
        create_test_proto_data(vec!["small", &"x".repeat(2000), "medium", &"y".repeat(100)]);
    let mut data = ResponseData::from_bytes(test_data);
    let mut reader = ProtoStreamReader::new(&mut data);

    let msg1 = reader.read_next_delimited_proto().unwrap();
    assert_eq!(
        TestMessage::decode_length_delimited(msg1.as_ref())
            .unwrap()
            .content,
        "small"
    );

    let msg2 = reader.read_next_delimited_proto().unwrap();
    assert_eq!(
        TestMessage::decode_length_delimited(msg2.as_ref())
            .unwrap()
            .content
            .len(),
        2000
    );

    let msg3 = reader.read_next_delimited_proto().unwrap();
    assert_eq!(
        TestMessage::decode_length_delimited(msg3.as_ref())
            .unwrap()
            .content,
        "medium"
    );

    let msg4 = reader.read_next_delimited_proto().unwrap();
    assert_eq!(
        TestMessage::decode_length_delimited(msg4.as_ref())
            .unwrap()
            .content
            .len(),
        100
    );
}
