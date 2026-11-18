//! # Attachments
//!
//! An `ATTACH` value (RFC 5545 section 3.8.1.1), told apart as bytes the
//! calendar carries or a reference to bytes elsewhere.
//!
//! Inline bytes are base64 nobody reads, so the form shows their media type
//! and their SHA-256 instead, and folds an unchanged digest back as the line
//! it came from. A `file` the document names is inlined, which is the only
//! way bytes enter a calendar through the form. tCard's media values take the
//! same shape.

use alloc::{format, string::String, vec::Vec};
use core::fmt::Write;

use base64_codec::codec::Base64Codec;
use sha2::{Digest, Sha256};

use crate::template::patch::Content;

/// The parameters saying how a value is encoded rather than what it is.
///
/// A line changing between inline and referenced drops the ones its new value
/// does not write.
pub const ENCODING_PARAMS: &[&str] = &["ENCODING", "VALUE", "FMTTYPE"];

/// What an attachment line holds.
pub enum Media<'a> {
    /// Bytes the calendar carries, named by their digest.
    Embedded {
        /// The media type the line declares, when it declares one.
        mediatype: Option<String>,
        /// The SHA-256 of the decoded bytes, in lowercase hex.
        sha256: String,
    },
    /// A reference, as the calendar wrote it.
    Uri(&'a str),
}

impl<'a> Media<'a> {
    /// Read a content line as inline bytes or as a reference.
    ///
    /// Inline bytes that do not decode are shown as the reference they then
    /// are, which folds back to the same line.
    pub fn of(line: &'a str) -> Self {
        let content = Content(line);

        match inline(&content) {
            Some((mediatype, bytes)) => Media::Embedded {
                mediatype,
                sha256: sha256(&bytes),
            },
            None => Media::Uri(content.value()),
        }
    }
}

/// A line inlining `bytes` as base64 binary, `FMTTYPE` read from them.
///
/// The parameters of the line it replaces are kept by the patch it goes
/// through, but for [`ENCODING_PARAMS`], which this line writes anew.
pub fn inlined(name: &str, bytes: &[u8]) -> String {
    let mut data = Vec::new();
    Base64Codec::default().encode_vec(bytes, &mut data);
    let data = String::from_utf8(data).unwrap_or_default();
    let mediatype = sniff(bytes);

    format!("{name};FMTTYPE={mediatype};ENCODING=BASE64;VALUE=BINARY:{data}")
}

/// A line referencing `uri`.
///
/// A line already holding a reference keeps its head, `FMTTYPE` included;
/// one that held bytes loses the parameters saying how they were encoded.
pub fn referenced(name: &str, original: Option<&String>, uri: &str) -> String {
    match original.filter(|line| matches!(Media::of(line), Media::Uri(_))) {
        Some(line) => format!("{}:{uri}", Content(line).head()),
        None => format!("{name}:{uri}"),
    }
}

/// The lowercase hex SHA-256 of some bytes, what `sha256sum` prints.
pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// The media type and the bytes a line carries inline, if it does.
///
/// RFC 5545 writes them as base64 under `ENCODING=BASE64` and `VALUE=BINARY`,
/// the media type in `FMTTYPE`. Some exports write an RFC 2397 `data:` URI
/// instead, which carries both.
fn inline(line: &Content<'_>) -> Option<(Option<String>, Vec<u8>)> {
    let value = line.value();

    if value
        .get(..5)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("data:"))
    {
        let (header, data) = value[5..].split_once(',')?;
        let base64 = header
            .rsplit(';')
            .next()
            .is_some_and(|last| last.eq_ignore_ascii_case("base64"));
        let mediatype = header
            .split(';')
            .next()
            .filter(|mediatype| mediatype.contains('/'))
            .map(str::to_ascii_lowercase);

        let bytes = match base64 {
            true => decode(data)?,
            false => percent_decoded(data),
        };

        return Some((mediatype, bytes));
    }

    let encoded = line
        .param("ENCODING")
        .is_some_and(|encoding| encoding.eq_ignore_ascii_case("BASE64"));

    if !encoded {
        return None;
    }

    let mediatype = line.param("FMTTYPE").map(str::to_ascii_lowercase);
    Some((mediatype, decode(value)?))
}

/// Base64 decoded, `None` when it holds a stray byte.
fn decode(data: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let clean = Base64Codec::default().decode_vec(data.as_bytes(), &mut bytes);
    clean.then_some(bytes)
}

/// A `data:` URI's payload written without base64, its `%XX` escapes undone.
fn percent_decoded(data: &str) -> Vec<u8> {
    let bytes = data.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;

    while at < bytes.len() {
        let hex = bytes
            .get(at + 1..at + 3)
            .and_then(|hex| core::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());

        match (bytes[at], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                at += 3;
            }
            (byte, _) => {
                out.push(byte);
                at += 1;
            }
        }
    }

    out
}

/// The media type some bytes open with, `application/octet-stream` when
/// unknown.
///
/// The same table as tCard's, so both twins name a file alike. An armoured
/// block, or a binary public key packet in the old (0x98-0x9B) or the new
/// (0xC6) framing of RFC 9580 section 4.2, is an OpenPGP key.
fn sniff(bytes: &[u8]) -> &'static str {
    match bytes {
        [0xFF, 0xD8, 0xFF, ..] => "image/jpeg",
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [b'G', b'I', b'F', b'8', ..] => "image/gif",
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => "image/webp",
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'A',
            b'V',
            b'E',
            ..,
        ] => "audio/wav",
        [b'O', b'g', b'g', b'S', ..] => "audio/ogg",
        [b'f', b'L', b'a', b'C', ..] => "audio/flac",
        [b'I', b'D', b'3', ..] | [0xFF, 0xF2 | 0xF3 | 0xFB, ..] => "audio/mpeg",
        [
            b'-',
            b'-',
            b'-',
            b'-',
            b'-',
            b'B',
            b'E',
            b'G',
            b'I',
            b'N',
            b' ',
            b'P',
            b'G',
            b'P',
            ..,
        ]
        | [0x98..=0x9B | 0xC6, ..] => "application/pgp-keys",
        [0x30, 0x82, ..] => "application/pkix-cert",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use crate::template::media::{Media, inlined, referenced, sha256};

    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0];

    fn embedded(line: &str) -> (Option<String>, String) {
        match Media::of(line) {
            Media::Embedded { mediatype, sha256 } => (mediatype, sha256),
            Media::Uri(uri) => panic!("read as uri {uri}"),
        }
    }

    #[test]
    fn both_inline_forms_carry_the_same_bytes() {
        let digest = (Some("image/jpeg".into()), sha256(JPEG));

        for line in [
            "ATTACH;FMTTYPE=image/jpeg;ENCODING=BASE64;VALUE=BINARY:/9j/4A==",
            "ATTACH;VALUE=BINARY;ENCODING=base64;FMTTYPE=IMAGE/JPEG:/9j/4A==",
            "ATTACH:data:image/jpeg;base64,/9j/4A==",
        ] {
            assert_eq!(embedded(line), digest, "{line}");
        }
    }

    #[test]
    fn a_digest_is_what_sha256sum_prints() {
        assert_eq!(
            sha256(b"foo"),
            "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae"
        );
        assert_eq!(embedded("ATTACH:data:,f%6Fo"), (None, sha256(b"foo")));
    }

    #[test]
    fn a_reference_is_kept_as_written() {
        for line in [
            "ATTACH;FMTTYPE=application/pdf:https://example.com/a,b.pdf",
            "ATTACH:CID:jsmith.part3.960817T083000.xyzMail@example.com",
            "ATTACH;ENCODING=BASE64;VALUE=BINARY:not base64!",
            "ATTACH;VALUE=BINARY:/9j/4A==",
        ] {
            assert!(matches!(Media::of(line), Media::Uri(_)), "{line}");
        }
    }

    #[test]
    fn a_file_is_inlined_as_base64_binary() {
        assert_eq!(
            inlined("ATTACH", JPEG),
            "ATTACH;FMTTYPE=image/jpeg;ENCODING=BASE64;VALUE=BINARY:/9j/4A=="
        );
        assert_eq!(
            inlined("ATTACH", b"?"),
            "ATTACH;FMTTYPE=application/octet-stream;ENCODING=BASE64;VALUE=BINARY:Pw=="
        );
    }

    #[test]
    fn a_reference_keeps_the_head_of_a_reference_only() {
        let held = String::from("ATTACH;FMTTYPE=application/pdf;X-LABEL=a:https://x");
        assert_eq!(
            referenced("ATTACH", Some(&held), "https://y"),
            "ATTACH;FMTTYPE=application/pdf;X-LABEL=a:https://y"
        );

        let bytes = String::from("ATTACH;FMTTYPE=image/jpeg;ENCODING=BASE64;VALUE=BINARY:/9j/4A==");
        assert_eq!(
            referenced("ATTACH", Some(&bytes), "https://y"),
            "ATTACH:https://y"
        );
    }
}
