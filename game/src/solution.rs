use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use flate2::read::ZlibDecoder;
use serde::Deserialize;
use std::io::Read;

#[cfg(any(test, target_arch = "wasm32"))]
use {flate2::Compression, flate2::write::ZlibEncoder, serde::Serialize, std::io::Write};

use crate::game::Action;

#[cfg(any(test, target_arch = "wasm32"))]
#[derive(Serialize)]
struct Solution<'a> {
    level: &'a str,
    grid: &'a str,
    actions: &'a [Vec<Action>],
}

#[derive(Deserialize)]
pub struct DecodedSolution {
    pub level: String,
    pub grid: String,
    pub actions: Vec<Vec<Action>>,
}

/// Encode a solution as zlib-compressed, base64-encoded JSON.
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) fn encode_solution(
    level_name: &str,
    initial_grid_csv: &str,
    action_history: &[Vec<Action>],
) -> String {
    let solution = Solution {
        level: level_name,
        grid: initial_grid_csv,
        actions: action_history,
    };

    let json = serde_json::to_string(&solution).expect("solution should serialize");

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(json.as_bytes())
        .expect("zlib write should succeed");
    let compressed = encoder.finish().expect("zlib finish should succeed");

    STANDARD.encode(&compressed)
}

/// Decode a base64-encoded, zlib-compressed JSON solution string.
pub fn decode_solution(encoded: &str) -> DecodedSolution {
    let compressed = STANDARD.decode(encoded).expect("invalid base64");
    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut json_str = String::new();
    decoder
        .read_to_string(&mut json_str)
        .expect("invalid zlib data");
    serde_json::from_str(&json_str).expect("invalid solution JSON")
}

/// Build a mailto URL for emailing a solution.
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) fn mailto_url(level_name: &str, encoded_solution: &str) -> String {
    let subject = percent_encode(&format!("Solution for {}", level_name));
    let body = percent_encode(encoded_solution);
    format!("mailto:dnspies@gmail.com?subject={subject}&body={body}")
}

#[cfg(any(test, target_arch = "wasm32"))]
fn percent_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::direction::Dir4;

    #[test]
    fn encode_decode_roundtrip() {
        let actions = vec![vec![Action::Move(Dir4::North)], vec![Action::Stall]];
        let encoded = encode_solution("test_level", ".,.\n▼,.", &actions);
        let decoded = decode_solution(&encoded);

        assert_eq!(decoded.level, "test_level");
        assert_eq!(decoded.grid, ".,.\n▼,.");
        assert_eq!(decoded.actions, actions);
    }

    #[test]
    fn mailto_url_structure() {
        let url = mailto_url("my level", "abc123==");
        assert!(url.starts_with("mailto:dnspies@gmail.com?subject="));
        assert!(url.contains("body=abc123%3D%3D"));
    }
}
