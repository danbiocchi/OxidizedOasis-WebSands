use rand::{thread_rng, Rng};
use uuid::Uuid;

pub fn generate_random_string(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                           abcdefghijklmnopqrstuvwxyz\
                           0123456789";
    let mut rng = thread_rng();

    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub fn generate_secure_token() -> String {
    Uuid::new_v4().to_string().replace("-", "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_generate_random_string_length() {
        assert_eq!(generate_random_string(0).len(), 0);
        assert_eq!(generate_random_string(10).len(), 10);
        assert_eq!(generate_random_string(100).len(), 100);
    }

    #[test]
    fn test_generate_random_string_charset() {
        let random_string = generate_random_string(50);
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                               abcdefghijklmnopqrstuvwxyz\
                               0123456789";
        let charset_set: HashSet<char> = CHARSET.iter().map(|&c| c as char).collect();

        for char_code in random_string.chars() {
            assert!(charset_set.contains(&char_code), "Character {} not in charset", char_code);
        }
    }

    #[test]
    fn test_generate_random_string_empty() {
        let random_string = generate_random_string(0);
        assert_eq!(random_string, "");
    }

    #[test]
    fn test_generate_random_string_different_lengths() {
        for len in [1, 5, 20, 50] {
            let result = generate_random_string(len);
            assert_eq!(result.len(), len);
        }
    }

    #[test]
    fn test_generate_random_strings_are_different() {
        let str1 = generate_random_string(20);
        let str2 = generate_random_string(20);
        
        // With high probability, two random strings should be different
        assert_ne!(str1, str2);
    }

    #[test]
    fn test_generate_secure_token_format() {
        let token = generate_secure_token();
        // UUID v4 without hyphens is 32 characters long
        assert_eq!(token.len(), 32);
        assert!(!token.contains("-"));
        // Check if it's a valid hex string (UUIDs are hex)
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_secure_token_uniqueness() {
        let token1 = generate_secure_token();
        let token2 = generate_secure_token();
        assert_ne!(token1, token2, "Generated tokens should be unique");
    }
}
