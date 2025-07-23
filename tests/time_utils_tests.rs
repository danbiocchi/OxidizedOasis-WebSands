//! Unit tests for time utilities

use oxidizedoasis_websands::common::utils::time::{is_expired, add_hours, format_timestamp};
use chrono::{DateTime, Utc, Duration};

#[test]
fn test_is_expired() {
    // Past time should be expired
    let past_time = Utc::now() - Duration::hours(1);
    assert!(is_expired(&past_time));
    
    // Future time should not be expired
    let future_time = Utc::now() + Duration::hours(1);
    assert!(!is_expired(&future_time));
    
    // Future time (even slightly) should not be expired
    let slightly_future = Utc::now() + Duration::milliseconds(100);
    assert!(!is_expired(&slightly_future));
}

#[test]
fn test_add_hours() {
    let base_time = Utc::now();
    
    // Add positive hours
    let future_time = add_hours(2);
    assert!(future_time > base_time);
    
    // Add zero hours should be current or very close
    let zero_time = add_hours(0);
    let diff = zero_time.signed_duration_since(base_time).num_seconds().abs();
    assert!(diff < 2); // Within 2 seconds
    
    // Add negative hours (past)
    let past_time = add_hours(-1);
    assert!(past_time < base_time);
}

#[test]
fn test_format_timestamp() {
    // Create a specific timestamp for consistent testing
    let timestamp = DateTime::parse_from_rfc3339("2023-12-25T15:30:45Z").unwrap().with_timezone(&Utc);
    
    let formatted = format_timestamp(timestamp);
    
    // Check that we get a non-empty string
    assert!(!formatted.is_empty());
    
    // Check that it contains the year
    assert!(formatted.contains("2023"));
}

#[test]
fn test_format_timestamp_current() {
    let now = Utc::now();
    let formatted = format_timestamp(now);
    
    // Should be a non-empty string
    assert!(!formatted.is_empty());
    
    // Should contain current year
    let current_year = now.format("%Y").to_string();
    assert!(formatted.contains(&current_year));
}

#[test]
fn test_add_hours_large_values() {
    let base_time = Utc::now();
    
    // Add 24 hours (1 day)
    let day_later = add_hours(24);
    let expected_diff = Duration::hours(24);
    let actual_diff = day_later.signed_duration_since(base_time);
    
    // Should be approximately 24 hours (within a few seconds)
    let diff_seconds = (actual_diff - expected_diff).num_seconds().abs();
    assert!(diff_seconds < 5);
}