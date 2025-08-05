pub enum EmailTemplate {
    Verification {
        verification_url: String,
        app_name: String,
    },
    PasswordReset {
        reset_url: String,
        app_name: String,
    },
}

impl EmailTemplate {
    pub fn render(&self) -> String {
        match self {
            EmailTemplate::Verification { verification_url, app_name } => {
                format!(
                    r#"
                    <!DOCTYPE html>
                    <html>
                    <head>
                        <meta charset="utf-8">
                    </head>
                    <body style="margin: 0; padding: 0; background-color: hsl(210, 25%, 8%); color: hsl(0, 0%, 100%); font-family: Arial, sans-serif;">
                        <table width="100%" cellpadding="0" cellspacing="0" style="background-color: hsl(210, 25%, 8%);">
                            <tr>
                                <td align="center" style="padding: 40px 0;">
                                    <table width="600" cellpadding="0" cellspacing="0" style="background-color: hsl(210, 18%, 12%); border-radius: 8px; box-shadow: 0 4px 12px rgba(0,0,0,0.5);">
                                        <tr>
                                            <td align="center" style="padding: 40px 40px 30px 40px;">
                                                <svg width="200" height="100" viewBox="0 0 200 100">
                                                    <g transform="translate(40,0)">
                                                        <rect fill="hsl(221, 83%, 53%)" x="0" y="20" width="10" height="10"/>
                                                        <rect fill="hsl(221, 83%, 53%)" x="20" y="10" width="10" height="10"/>
                                                        <rect fill="hsl(221, 83%, 53%)" x="40" y="30" width="10" height="10"/>
                                                        <rect fill="hsl(221, 83%, 53%)" x="60" y="15" width="10" height="10"/>
                                                        <rect fill="hsl(221, 83%, 53%)" x="80" y="25" width="10" height="10"/>
                                                        <rect fill="hsl(221, 83%, 53%)" x="100" y="5" width="10" height="10"/>
                                                    </g>
                                                    <text x="100" y="80" text-anchor="middle" 
                                                          style="font-family: Arial; font-size: 16px; fill: hsl(221, 83%, 53%); letter-spacing: 2px;">{app_name}</text>
                                                </svg>
                                                <h1 style="color: hsl(221, 83%, 53%); margin: 20px 0; font-size: 24px; font-weight: normal;">Verify Your Email</h1>
                                                <p style="color: hsl(0, 0%, 100%); margin: 0 0 30px 0; line-height: 24px;">Thank you for signing up with {app_name}! Please click the button below to verify your email address:</p>
                                                <table cellpadding="0" cellspacing="0" style="margin: 30px 0;">
                                                    <tr>
                                                        <td style="background-color: hsl(221, 83%, 53%); border-radius: 4px;">
                                                            <a href="{verification_url}" style="display: block; padding: 15px 30px; color: hsl(210, 25%, 8%); text-decoration: none; font-weight: bold;">Verify Email</a>
                                                        </td>
                                                    </tr>
                                                </table>
                                                <p style="color: hsl(0, 0%, 80%); margin: 0 0 10px 0; font-size: 14px;">If the button doesn't work, you can copy and paste the following link into your browser:</p>
                                                <p style="background-color: hsl(210, 12%, 19%); padding: 15px; border-radius: 4px; word-break: break-all; margin: 0 0 20px 0;"><a href="{verification_url}" style="color: hsl(221, 83%, 53%); text-decoration: none;">{verification_url}</a></p>
                                                <p style="color: hsl(0, 0%, 60%); margin: 20px 0 0 0; font-size: 14px;">This link will expire in 24 hours.</p>
                                                <p style="color: hsl(0, 0%, 60%); margin: 10px 0 0 0; font-size: 14px;">If you didn't sign up for an account, you can safely ignore this email.</p>
                                            </td>
                                        </tr>
                                    </table>
                                </td>
                            </tr>
                        </table>
                    </body>
                    </html>
                    "#
                )
            }
            EmailTemplate::PasswordReset { reset_url, app_name } => {
                format!(
                    r#"
                    <!DOCTYPE html>
                    <html>
                    <head>
                        <meta charset="utf-8">
                    </head>
                    <body style="margin: 0; padding: 0; background-color: hsl(210, 25%, 8%); color: hsl(0, 0%, 100%); font-family: Arial, sans-serif;">
                        <table width="100%" cellpadding="0" cellspacing="0" style="background-color: hsl(210, 25%, 8%);">
                            <tr>
                                <td align="center" style="padding: 40px 0;">
                                    <table width="600" cellpadding="0" cellspacing="0" style="background-color: hsl(210, 18%, 12%); border-radius: 8px; box-shadow: 0 4px 12px rgba(0,0,0,0.5);">
                                        <tr>
                                            <td align="center" style="padding: 40px 40px 30px 40px;">
                                                <svg width="200" height="100" viewBox="0 0 200 100">
                                                    <g transform="translate(40,0)">
                                                        <rect fill="hsl(221, 83%, 53%)" x="0" y="20" width="10" height="10"/>
                                                        <rect fill="hsl(221, 83%, 53%)" x="20" y="10" width="10" height="10"/>
                                                        <rect fill="hsl(221, 83%, 53%)" x="40" y="30" width="10" height="10"/>
                                                        <rect fill="hsl(221, 83%, 53%)" x="60" y="15" width="10" height="10"/>
                                                        <rect fill="hsl(221, 83%, 53%)" x="80" y="25" width="10" height="10"/>
                                                        <rect fill="hsl(221, 83%, 53%)" x="100" y="5" width="10" height="10"/>
                                                    </g>
                                                    <text x="100" y="80" text-anchor="middle" 
                                                          style="font-family: Arial; font-size: 16px; fill: hsl(221, 83%, 53%); letter-spacing: 2px;">{app_name}</text>
                                                </svg>
                                                
                                                <h1 style="color: hsl(221, 83%, 53%); margin: 20px 0; font-size: 24px; font-weight: normal;">Reset Your Password</h1>
                                                <p style="color: hsl(0, 0%, 100%); margin: 0 0 30px 0; line-height: 24px;">We received a request to reset your {app_name} password. Click the button below to choose a new password:</p>
                                                <table cellpadding="0" cellspacing="0" style="margin: 30px 0;">
                                                    <tr>
                                                        <td style="background-color: hsl(221, 83%, 53%); border-radius: 4px;">
                                                            <a href="{reset_url}" style="display: block; padding: 15px 30px; color: hsl(210, 25%, 8%); text-decoration: none; font-weight: bold;">Reset Password</a>
                                                        </td>
                                                    </tr>
                                                </table>
                                                <p style="color: hsl(0, 0%, 80%); margin: 0 0 10px 0; font-size: 14px;">If the button doesn't work, you can copy and paste the following link into your browser:</p>
                                                <p style="background-color: hsl(210, 12%, 19%); padding: 15px; border-radius: 4px; word-break: break-all; margin: 0 0 20px 0;"><a href="{reset_url}" style="color: hsl(221, 83%, 53%); text-decoration: none;">{reset_url}</a></p>
                                                <p style="color: hsl(0, 0%, 60%); margin: 20px 0 0 0; font-size: 14px;">This link will expire in 1 hour for security reasons.</p>
                                                <p style="color: hsl(0, 0%, 60%); margin: 10px 0 0 0; font-size: 14px;">If you didn't request a password reset, you can safely ignore this email. Your password will remain unchanged.</p>
                                            </td>
                                        </tr>
                                    </table>
                                </td>
                            </tr>
                        </table>
                    </body>
                    </html>
                    "#
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_template_render() {
        let template = EmailTemplate::Verification {
            verification_url: "https://example.com/verify?token=abc123".to_string(),
            app_name: "TestApp".to_string(),
        };
        
        let rendered = template.render();
        
        // Check that the rendered HTML contains expected elements
        assert!(rendered.contains("<!DOCTYPE html>"));
        assert!(rendered.contains("Verify Your Email"));
        assert!(rendered.contains("https://example.com/verify?token=abc123"));
        assert!(rendered.contains("TestApp"));
        assert!(rendered.contains("Verify Email"));
        assert!(rendered.contains("This link will expire in 24 hours"));
        assert!(rendered.contains("If you didn't sign up for an account"));
        
        // Check that both URL placeholders are replaced
        assert!(rendered.matches("https://example.com/verify?token=abc123").count() >= 2);
        
        // Check that both app name placeholders are replaced
        assert!(rendered.matches("TestApp").count() >= 2);
    }

    #[test]
    fn test_password_reset_template_render() {
        let template = EmailTemplate::PasswordReset {
            reset_url: "https://example.com/reset?token=xyz789".to_string(),
            app_name: "MyApplication".to_string(),
        };
        
        let rendered = template.render();
        
        // Check that the rendered HTML contains expected elements
        assert!(rendered.contains("<!DOCTYPE html>"));
        assert!(rendered.contains("Reset Your Password"));
        assert!(rendered.contains("https://example.com/reset?token=xyz789"));
        assert!(rendered.contains("MyApplication"));
        assert!(rendered.contains("Reset Password"));
        assert!(rendered.contains("This link will expire in 1 hour"));
        assert!(rendered.contains("If you didn't request a password reset"));
        
        // Check that both URL placeholders are replaced
        assert!(rendered.matches("https://example.com/reset?token=xyz789").count() >= 2);
        
        // Check that both app name placeholders are replaced
        assert!(rendered.matches("MyApplication").count() >= 2);
    }

    #[test]
    fn test_verification_template_with_special_characters() {
        let template = EmailTemplate::Verification {
            verification_url: "https://example.com/verify?token=abc&123".to_string(),
            app_name: "Test & App".to_string(),
        };
        
        let rendered = template.render();
        
        // Ensure special characters are preserved in URLs and text
        assert!(rendered.contains("https://example.com/verify?token=abc&123"));
        assert!(rendered.contains("Test & App"));
    }

    #[test]
    fn test_password_reset_template_with_special_characters() {
        let template = EmailTemplate::PasswordReset {
            reset_url: "https://example.com/reset?token=xyz&789".to_string(),
            app_name: "My App & Co".to_string(),
        };
        
        let rendered = template.render();
        
        // Ensure special characters are preserved in URLs and text
        assert!(rendered.contains("https://example.com/reset?token=xyz&789"));
        assert!(rendered.contains("My App & Co"));
    }

    #[test]
    fn test_verification_template_structure() {
        let template = EmailTemplate::Verification {
            verification_url: "http://localhost:3000/verify".to_string(),
            app_name: "DevApp".to_string(),
        };
        
        let rendered = template.render();
        
        // Test HTML structure elements
        assert!(rendered.contains("<html>"));
        assert!(rendered.contains("</html>"));
        assert!(rendered.contains("<head>"));
        assert!(rendered.contains("</head>"));
        assert!(rendered.contains("<body"));
        assert!(rendered.contains("</body>"));
        assert!(rendered.contains("<table"));
        assert!(rendered.contains("</table>"));
        assert!(rendered.contains("<h1"));
        assert!(rendered.contains("</h1>"));
        
        // Test CSS styling presence
        assert!(rendered.contains("background-color:"));
        assert!(rendered.contains("color:"));
        assert!(rendered.contains("font-family:"));
    }

    #[test]
    fn test_password_reset_template_structure() {
        let template = EmailTemplate::PasswordReset {
            reset_url: "http://localhost:3000/reset".to_string(),
            app_name: "DevApp".to_string(),
        };
        
        let rendered = template.render();
        
        // Test HTML structure elements
        assert!(rendered.contains("<html>"));
        assert!(rendered.contains("</html>"));
        assert!(rendered.contains("<head>"));
        assert!(rendered.contains("</head>"));
        assert!(rendered.contains("<body"));
        assert!(rendered.contains("</body>"));
        assert!(rendered.contains("<table"));
        assert!(rendered.contains("</table>"));
        assert!(rendered.contains("<h1"));
        assert!(rendered.contains("</h1>"));
        
        // Test CSS styling presence
        assert!(rendered.contains("background-color:"));
        assert!(rendered.contains("color:"));
        assert!(rendered.contains("font-family:"));
    }

    #[test]
    fn test_template_differences() {
        let verification_template = EmailTemplate::Verification {
            verification_url: "https://example.com/verify".to_string(),
            app_name: "TestApp".to_string(),
        };
        
        let reset_template = EmailTemplate::PasswordReset {
            reset_url: "https://example.com/reset".to_string(),
            app_name: "TestApp".to_string(),
        };
        
        let verification_rendered = verification_template.render();
        let reset_rendered = reset_template.render();
        
        // Ensure templates are different
        assert_ne!(verification_rendered, reset_rendered);
        
        // Ensure verification template has verification-specific content
        assert!(verification_rendered.contains("Verify Your Email"));
        assert!(verification_rendered.contains("verify"));
        assert!(verification_rendered.contains("24 hours"));
        
        // Ensure reset template has reset-specific content
        assert!(reset_rendered.contains("Reset Your Password"));
        assert!(reset_rendered.contains("reset"));
        assert!(reset_rendered.contains("1 hour"));
    }
}
