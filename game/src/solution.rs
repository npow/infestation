use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

use crate::game::Action;

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
}
