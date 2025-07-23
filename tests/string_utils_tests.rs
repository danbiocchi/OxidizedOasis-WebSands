//! Unit tests for string utilities

use oxidizedoasis_websands::common::utils::string::generate_random_string;

#[test]
fn test_generate_random_string() {
    let length = 10;
    let result = generate_random_string(length);
    
    assert_eq!(result.len(), length);
    assert!(result.chars().all(|c| c.is_alphanumeric()));
}

#[test]
fn test_generate_random_string_different_lengths() {
    for len in [1, 5, 20, 50] {
        let result = generate_random_string(len);
        assert_eq!(result.len(), len);
    }
}

#[test]
fn test_generate_random_string_empty() {
    let result = generate_random_string(0);
    assert!(result.is_empty());
}

#[test]
fn test_generate_random_strings_are_different() {
    let str1 = generate_random_string(20);
    let str2 = generate_random_string(20);
    
    // With high probability, two random strings should be different
    assert_ne!(str1, str2);
}