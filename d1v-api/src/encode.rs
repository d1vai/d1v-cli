use itertools::Itertools;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

pub fn encode_segment(s: impl AsRef<str>) -> String {
    utf8_percent_encode(s.as_ref(), PATH_SEGMENT).to_string()
}

pub fn encode_path(path: impl AsRef<str>) -> String {
    path.as_ref()
        .split('/')
        .map(|s| utf8_percent_encode(s, PATH_SEGMENT))
        .join("/")
}
