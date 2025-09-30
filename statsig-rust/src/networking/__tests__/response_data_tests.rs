use std::io::Cursor;

use crate::networking::ResponseData;

#[test]
fn test_from_bytes() {
    let mut response_data = ResponseData::from_bytes(b"test".to_vec());
    assert_eq!(response_data.read_to_string().unwrap(), "test");
}

#[test]
fn test_from_stream() {
    let stream = Cursor::new(b"test");
    let mut response_data = ResponseData::from_stream(Box::new(stream));
    assert_eq!(response_data.read_to_string().unwrap(), "test");
}

#[test]
fn test_deserialize_into() {
    let mut response_data = ResponseData::from_bytes(r#""test""#.to_string().as_bytes().to_vec());
    let result = response_data.deserialize_into::<String>().unwrap();
    assert_eq!(result, "test");
}

#[test]
fn test_deserialize_into_error() {
    let mut response_data = ResponseData::from_bytes(b"test".to_vec());
    let result = response_data.deserialize_into::<String>();
    assert!(result.is_err());
}

#[test]
fn test_deserialize_then_read() {
    let mut response_data = ResponseData::from_bytes(r#""test""#.to_string().as_bytes().to_vec());
    let result = response_data.deserialize_into::<String>().unwrap();
    assert_eq!(result, "test");
    assert_eq!(response_data.read_to_string().unwrap(), "\"test\"");
}

#[test]
fn test_double_deserialize() {
    let mut response_data = ResponseData::from_bytes(r#""test""#.to_string().as_bytes().to_vec());
    let result = response_data.deserialize_into::<String>().unwrap();
    assert_eq!(result, "test");

    let result2 = response_data.deserialize_into::<String>().unwrap();
    assert_eq!(result2, "test");
}

#[test]
fn test_deserialize_into_in_place() {
    let mut response_data = ResponseData::from_bytes(r#""test""#.to_string().as_bytes().to_vec());
    let mut result = String::new();
    response_data.deserialize_in_place(&mut result).unwrap();
    assert_eq!(result, "test");
}

#[test]
fn test_double_deserialize_in_place() {
    let mut response_data = ResponseData::from_bytes(r#""test""#.to_string().as_bytes().to_vec());
    let mut result = String::new();
    response_data.deserialize_in_place(&mut result).unwrap();
    assert_eq!(result, "test");

    let mut result2 = String::new();
    response_data.deserialize_in_place(&mut result2).unwrap();
    assert_eq!(result2, "test");
}

#[test]
fn test_read_deserialize_and_deserialize_in_place() {
    let mut response_data = ResponseData::from_bytes(r#""test""#.to_string().as_bytes().to_vec());

    let read_result = response_data.read_to_string().unwrap();
    assert_eq!(read_result, "\"test\"");

    let deserialize_result = response_data.deserialize_into::<String>().unwrap();
    assert_eq!(deserialize_result, "test");

    let mut place = String::new();
    response_data.deserialize_in_place(&mut place).unwrap();
    assert_eq!(place, "test");
}

#[test]
fn test_deserialization_failure_retry() {
    let mut response_data = ResponseData::from_bytes(b"false".to_vec());
    let result = response_data.deserialize_into::<String>();
    assert!(result.is_err());

    let result2 = response_data.deserialize_into::<bool>();
    assert!(result2.is_ok_and(|bool_result| !bool_result));
}

#[test]
fn test_deserialization_failure_retry_in_place() {
    let mut response_data = ResponseData::from_bytes(b"false".to_vec());
    let mut place = String::new();
    let result = response_data.deserialize_in_place::<String>(&mut place);
    assert!(result.is_err());

    let result2 = response_data.deserialize_into::<bool>();
    assert!(result2.is_ok_and(|bool_result| !bool_result));
}
