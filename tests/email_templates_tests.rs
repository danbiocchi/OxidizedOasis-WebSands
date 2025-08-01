//! Tests for src/core/email/templates.rs - Email template rendering
//! 
//! This test file covers the EmailTemplate enum and its rendering functionality
//! for verification and password reset emails.

use oxidizedoasis_websands::core::email::templates::EmailTemplate;

#[cfg(test)]
mod email_template_tests {
    use super::*;

    /// Test verification email template rendering
    #[test]
    fn test_verification_email_template_render() {
        let verification_url = "https://example.com/verify?token=abc123".to_string();
        let app_name = "TestApp".to_string();
        
        let template = EmailTemplate::Verification {
            verification_url: verification_url.clone(),
            app_name: app_name.clone(),
        };

        let rendered = template.render();
        
        // Check that the rendered output contains expected elements
        assert!(rendered.contains("<!DOCTYPE html>"));
        assert!(rendered.contains("<html>"));
        assert!(rendered.contains("</html>"));
        assert!(rendered.contains("Verify Your Email"));
        assert!(rendered.contains(&verification_url));
        assert!(rendered.contains(&app_name));
        assert!(rendered.contains("Thank you for signing up"));
        assert!(rendered.contains("Verify Email"));
        assert!(rendered.contains("This link will expire in 24 hours"));
        
        // Check for dark theme styling
        assert!(rendered.contains("background-color: hsl(210, 25%, 8%)"));
        assert!(rendered.contains("color: hsl(0, 0%, 100%)"));
        
        // Check for security-related content
        assert!(rendered.contains("If you didn't sign up for an account"));
    }

    /// Test password reset email template rendering
    #[test]
    fn test_password_reset_email_template_render() {
        let reset_url = "https://example.com/reset?token=xyz789".to_string();
        let app_name = "TestApp".to_string();
        
        let template = EmailTemplate::PasswordReset {
            reset_url: reset_url.clone(),
            app_name: app_name.clone(),
        };

        let rendered = template.render();
        
        // Check that the rendered output contains expected elements
        assert!(rendered.contains("<!DOCTYPE html>"));
        assert!(rendered.contains("<html>"));
        assert!(rendered.contains("</html>"));
        assert!(rendered.contains("Reset Your Password"));
        assert!(rendered.contains(&reset_url));
        assert!(rendered.contains(&app_name));
        assert!(rendered.contains("We received a request to reset"));
        assert!(rendered.contains("Reset Password"));
        assert!(rendered.contains("This link will expire in 1 hour"));
        
        // Check for dark theme styling
        assert!(rendered.contains("background-color: hsl(210, 25%, 8%)"));
        assert!(rendered.contains("color: hsl(0, 0%, 100%)"));
        
        // Check for security-related content
        assert!(rendered.contains("If you didn't request a password reset"));
        assert!(rendered.contains("Your password will remain unchanged"));
    }

    /// Test verification template with special characters
    #[test]
    fn test_verification_template_with_special_characters() {
        let verification_url = "https://example.com/verify?token=abc123&redirect=%2Fdashboard".to_string();
        let app_name = "Test & Co.".to_string();
        
        let template = EmailTemplate::Verification {
            verification_url: verification_url.clone(),
            app_name: app_name.clone(),
        };

        let rendered = template.render();
        
        // Verify that special characters are included correctly
        assert!(rendered.contains(&verification_url));
        assert!(rendered.contains("Test & Co."));
    }

    /// Test password reset template with special characters
    #[test]
    fn test_password_reset_template_with_special_characters() {
        let reset_url = "https://example.com/reset?token=xyz789&user=test%40example.com".to_string();
        let app_name = "Test & Co.".to_string();
        
        let template = EmailTemplate::PasswordReset {
            reset_url: reset_url.clone(),
            app_name: app_name.clone(),
        };

        let rendered = template.render();
        
        // Verify that special characters are included correctly
        assert!(rendered.contains(&reset_url));
        assert!(rendered.contains("Test & Co."));
    }

    /// Test template structure and HTML validity basics
    #[test]
    fn test_template_html_structure() {
        let template = EmailTemplate::Verification {
            verification_url: "https://test.com/verify".to_string(),
            app_name: "TestApp".to_string(),
        };

        let rendered = template.render();
        
        // Check basic HTML structure
        assert!(rendered.contains("<!DOCTYPE html>"));
        assert!(rendered.contains("<html>"));
        assert!(rendered.contains("<head>"));
        assert!(rendered.contains("<meta charset=\"utf-8\">"));
        assert!(rendered.contains("</head>"));
        assert!(rendered.contains("<body"));
        assert!(rendered.contains("</body>"));
        assert!(rendered.contains("</html>"));
        
        // Check for table-based layout (common in email templates)
        assert!(rendered.contains("<table"));
        assert!(rendered.contains("</table>"));
        assert!(rendered.contains("<tr>"));
        assert!(rendered.contains("</tr>"));
        assert!(rendered.contains("<td"));
        assert!(rendered.contains("</td>"));
    }

    /// Test template accessibility features
    #[test]
    fn test_template_accessibility_features() {
        let template = EmailTemplate::Verification {
            verification_url: "https://test.com/verify".to_string(),
            app_name: "TestApp".to_string(),
        };

        let rendered = template.render();
        
        // Check for alternative text options
        assert!(rendered.contains("If the button doesn't work"));
        assert!(rendered.contains("copy and paste the following link"));
        
        // Check for readable styling
        assert!(rendered.contains("font-family: Arial"));
        assert!(rendered.contains("line-height"));
    }

    /// Test template branding consistency
    #[test]
    fn test_template_branding_consistency() {
        let app_name = "BrandName".to_string();
        
        let verification_template = EmailTemplate::Verification {
            verification_url: "https://test.com/verify".to_string(),
            app_name: app_name.clone(),
        };

        let reset_template = EmailTemplate::PasswordReset {
            reset_url: "https://test.com/reset".to_string(),
            app_name: app_name.clone(),
        };

        let verification_rendered = verification_template.render();
        let reset_rendered = reset_template.render();
        
        // Both templates should contain the brand name
        assert!(verification_rendered.contains("BrandName"));
        assert!(reset_rendered.contains("BrandName"));
        
        // Both templates should have consistent styling
        let brand_color = "hsl(221, 83%, 53%)";
        assert!(verification_rendered.contains(brand_color));
        assert!(reset_rendered.contains(brand_color));
        
        // Both should have the same background styling
        let bg_color = "hsl(210, 25%, 8%)";
        assert!(verification_rendered.contains(bg_color));
        assert!(reset_rendered.contains(bg_color));
    }

    /// Test template with empty strings (edge case)
    #[test]
    fn test_template_with_empty_strings() {
        let template = EmailTemplate::Verification {
            verification_url: "".to_string(),
            app_name: "".to_string(),
        };

        let rendered = template.render();
        
        // Should still render valid HTML structure
        assert!(rendered.contains("<!DOCTYPE html>"));
        assert!(rendered.contains("Verify Your Email"));
        
        // Empty strings should be handled gracefully
        assert!(rendered.contains("href=\"\""));
    }

    /// Test template with very long strings (edge case)
    #[test]
    fn test_template_with_long_strings() {
        let long_url = format!("https://example.com/verify?token={}", "a".repeat(1000));
        let long_app_name = "A".repeat(100);
        
        let template = EmailTemplate::Verification {
            verification_url: long_url.clone(),
            app_name: long_app_name.clone(),
        };

        let rendered = template.render();
        
        // Should handle long strings without breaking
        assert!(rendered.contains(&long_url));
        assert!(rendered.contains(&long_app_name));
        assert!(rendered.contains("<!DOCTYPE html>"));
    }

    /// Test template rendering is deterministic
    #[test]
    fn test_template_rendering_deterministic() {
        let template = EmailTemplate::Verification {
            verification_url: "https://test.com/verify".to_string(),
            app_name: "TestApp".to_string(),
        };

        let rendered1 = template.render();
        let rendered2 = template.render();
        
        // Should produce identical output for identical input
        assert_eq!(rendered1, rendered2);
    }

    /// Test both template variants are distinct
    #[test]
    fn test_template_variants_are_distinct() {
        let verification_template = EmailTemplate::Verification {
            verification_url: "https://test.com/verify".to_string(),
            app_name: "TestApp".to_string(),
        };

        let reset_template = EmailTemplate::PasswordReset {
            reset_url: "https://test.com/reset".to_string(),
            app_name: "TestApp".to_string(),
        };

        let verification_rendered = verification_template.render();
        let reset_rendered = reset_template.render();
        
        // Should have different content
        assert_ne!(verification_rendered, reset_rendered);
        
        // Verification should have verification-specific content
        assert!(verification_rendered.contains("Verify Your Email"));
        assert!(!verification_rendered.contains("Reset Your Password"));
        
        // Reset should have reset-specific content
        assert!(reset_rendered.contains("Reset Your Password"));
        assert!(!reset_rendered.contains("Verify Your Email"));
    }
}