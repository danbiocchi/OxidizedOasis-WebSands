use serde::Serialize;

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            message: None,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: None,
            data: None,
            error: Some(message.into()),
        }
    }

    pub fn success_with_message(message: &str, data: T) -> Self {
        Self {
            success: true,
            message: Some(message.to_string()),
            data: Some(data),
            error: None,
        }
    }

    pub fn error_with_type(message: impl Into<String>, error_type: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
            data: None,
            error: Some(error_type.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[derive(Serialize, Debug, PartialEq)]
    struct TestData {
        id: u32,
        name: String,
    }

    #[test]
    fn test_api_response_success() {
        let test_data = TestData {
            id: 1,
            name: "Test User".to_string(),
        };

        let response = ApiResponse::success(test_data);

        assert_eq!(response.success, true);
        assert_eq!(response.message, None);
        assert!(response.data.is_some());
        assert_eq!(response.error, None);

        // Verify the data is correctly stored
        let data = response.data.unwrap();
        assert_eq!(data.id, 1);
        assert_eq!(data.name, "Test User");
    }

    #[test]
    fn test_api_response_error() {
        let error_message = "Something went wrong";
        let response: ApiResponse<String> = ApiResponse::error(error_message);

        assert_eq!(response.success, false);
        assert_eq!(response.message, None);
        assert_eq!(response.data, None);
        assert_eq!(response.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_api_response_error_with_string() {
        let error_message = String::from("Database connection failed");
        let response: ApiResponse<i32> = ApiResponse::error(error_message);

        assert_eq!(response.success, false);
        assert_eq!(response.message, None);
        assert_eq!(response.data, None);
        assert_eq!(response.error, Some("Database connection failed".to_string()));
    }

    #[test]
    fn test_api_response_success_with_message() {
        let test_data = TestData {
            id: 42,
            name: "John Doe".to_string(),
        };
        let message = "User created successfully";

        let response = ApiResponse::success_with_message(message, test_data);

        assert_eq!(response.success, true);
        assert_eq!(response.message, Some("User created successfully".to_string()));
        assert!(response.data.is_some());
        assert_eq!(response.error, None);

        let data = response.data.unwrap();
        assert_eq!(data.id, 42);
        assert_eq!(data.name, "John Doe");
    }

    #[test]
    fn test_api_response_error_with_type() {
        let message = "Validation failed";
        let error_type = "ValidationError";

        let response: ApiResponse<()> = ApiResponse::error_with_type(message, error_type);

        assert_eq!(response.success, false);
        assert_eq!(response.message, Some("Validation failed".to_string()));
        assert_eq!(response.data, None);
        assert_eq!(response.error, Some("ValidationError".to_string()));
    }

    #[test]
    fn test_api_response_error_with_type_string_inputs() {
        let message = String::from("Authentication failed");
        let error_type = String::from("AuthError");

        let response: ApiResponse<TestData> = ApiResponse::error_with_type(message, error_type);

        assert_eq!(response.success, false);
        assert_eq!(response.message, Some("Authentication failed".to_string()));
        assert_eq!(response.data, None);
        assert_eq!(response.error, Some("AuthError".to_string()));
    }

    #[test]
    fn test_api_response_serialization_success() {
        let test_data = TestData {
            id: 123,
            name: "Serialize Test".to_string(),
        };

        let response = ApiResponse::success(test_data);
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"id\":123"));
        assert!(json.contains("\"name\":\"Serialize Test\""));
        assert!(json.contains("\"message\":null"));
        assert!(json.contains("\"error\":null"));
    }

    #[test]
    fn test_api_response_serialization_error() {
        let response: ApiResponse<()> = ApiResponse::error("Test error");
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":\"Test error\""));
        assert!(json.contains("\"data\":null"));
        assert!(json.contains("\"message\":null"));
    }

    #[test]
    fn test_api_response_serialization_success_with_message() {
        let response = ApiResponse::success_with_message("Operation completed", "result_data");
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message\":\"Operation completed\""));
        assert!(json.contains("\"data\":\"result_data\""));
        assert!(json.contains("\"error\":null"));
    }

    #[test]
    fn test_api_response_serialization_error_with_type() {
        let response: ApiResponse<i32> = ApiResponse::error_with_type("Invalid input", "InputError");
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"message\":\"Invalid input\""));
        assert!(json.contains("\"error\":\"InputError\""));
        assert!(json.contains("\"data\":null"));
    }

    #[test]
    fn test_api_response_with_different_data_types() {
        // Test with integer
        let int_response = ApiResponse::success(42i32);
        assert_eq!(int_response.data, Some(42));

        // Test with string
        let string_response = ApiResponse::success("hello".to_string());
        assert_eq!(string_response.data, Some("hello".to_string()));

        // Test with vector
        let vec_response = ApiResponse::success(vec![1, 2, 3]);
        assert_eq!(vec_response.data, Some(vec![1, 2, 3]));

        // Test with boolean
        let bool_response = ApiResponse::success(true);
        assert_eq!(bool_response.data, Some(true));
    }

    #[test]
    fn test_api_response_empty_strings() {
        let response: ApiResponse<()> = ApiResponse::error("");
        assert_eq!(response.error, Some("".to_string()));

        let response2: ApiResponse<()> = ApiResponse::error_with_type("", "");
        assert_eq!(response2.message, Some("".to_string()));
        assert_eq!(response2.error, Some("".to_string()));

        let response3 = ApiResponse::success_with_message("", "test_data");
        assert_eq!(response3.message, Some("".to_string()));
    }

    #[test]
    fn test_api_response_option_fields() {
        let response = ApiResponse::success("data");
        
        // Test that Option fields work correctly
        assert!(response.message.is_none());
        assert!(response.data.is_some());
        assert!(response.error.is_none());

        let error_response: ApiResponse<()> = ApiResponse::error("error");
        assert!(error_response.message.is_none());
        assert!(error_response.data.is_none());
        assert!(error_response.error.is_some());
    }
}
