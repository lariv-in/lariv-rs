use rand::RngExt;

const CROCKFORD: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// Autogenerate a unique-looking room code (caller retries on DB collision).
pub fn generate_room_code(len: usize) -> String {
    let n = len.max(4).min(16);
    let mut rng = rand::rng();
    (0..n)
        .map(|_| {
            let idx = rng.random_range(0..CROCKFORD.len());
            CROCKFORD.get(idx).copied().unwrap_or(b'x') as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_length_and_alphabet() {
        let code = generate_room_code(8);
        assert_eq!(code.len(), 8);
        assert!(code.bytes().all(|b| CROCKFORD.contains(&b)));
    }
}
