use crate::networking::ResponseData;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecsResponseFormat {
    Json,
    PlainText,
    Protobuf,
    Unknown,
}

impl SpecsResponseFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::PlainText => "plain_text",
            Self::Protobuf => "protobuf",
            Self::Unknown => "unknown",
        }
    }
}

fn is_response_protobuf(response_data: &ResponseData) -> bool {
    let content_type = response_data.get_header_ref("content-type");
    if content_type.map(|s| s.as_str().contains("application/octet-stream")) != Some(true) {
        return false;
    }

    let content_encoding = response_data.get_header_ref("content-encoding");
    content_encoding.map(|s| s.as_str().contains("statsig-br")) == Some(true)
}

pub fn get_specs_response_format(response_data: &ResponseData) -> SpecsResponseFormat {
    if is_response_protobuf(response_data) {
        return SpecsResponseFormat::Protobuf;
    }

    let content_type = match response_data.get_header_ref("content-type") {
        Some(content_type) => content_type.to_ascii_lowercase(),
        None => return SpecsResponseFormat::Unknown,
    };

    if content_type.contains("application/json") || content_type.contains("+json") {
        return SpecsResponseFormat::Json;
    }

    if content_type.contains("text/plain") {
        return SpecsResponseFormat::PlainText;
    }

    SpecsResponseFormat::Unknown
}
