use yew::prelude::*;
use web_sys::HtmlInputElement;
use wasm_bindgen_futures::spawn_local;
use gloo::net::http::Request; // Import Request service directly
use serde::{Deserialize, Serialize}; // Import serde traits
use serde_json::{self, json}; // Import serde_json for parsing and json! macro
use crate::services::request::RequestBuilderExt;

// Helper Structs for API responses
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub is_email_verified: bool,
    pub role: String, // Uncommented and added
    pub is_active: bool, // Added is_active field
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UserResponseData {
   pub user: User,
   pub csrf_token: Option<String>, // If CSRF token is part of this response
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UserLoadResponse { // Wrapper for /api/cookie/users/me GET response
    pub success: bool,
    pub data: UserResponseData,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProfileUpdateResponse { // Wrapper for PUT /api/cookie/users/{id} response
    pub success: bool,
    pub message: String,
    pub data: UserResponseData, // Assuming the update returns the user and potentially a new CSRF
}


pub struct AccountSettings {
    username: String,
    email: String,
    original_email: String, // Added
    user_id: Option<String>, // Added
    csrf_token: Option<String>, // Added (though request.rs might handle it)
    is_loading: bool,
    error_message: Option<String>,
    success_message: Option<String>,
}

pub enum Msg {
    UserLoaded(Result<serde_json::Value, String>),
    ProfileUpdateResponse(Result<serde_json::Value, String>),
    UpdateUsername(String),
    UpdateEmail(String),
    SaveChanges,
    ClearMessages,
}

impl Component for AccountSettings {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        // Load user data when component is created using RequestInterceptor
        let link = ctx.link().clone();
        spawn_local(async move {
            gloo::console::log!("🔍 DEBUG: AccountSettings loading user data");
            
            let result = match crate::services::request::RequestInterceptor::get("/api/cookie/users/me")
                .send_with_retry().await {
                Ok(response) => {
                    gloo::console::log!("🔍 DEBUG: AccountSettings got response, parsing JSON");
                    response.json().await.map_err(|e| {
                        gloo::console::log!("🔍 DEBUG: JSON parse error:", &e.to_string());
                        e.to_string()
                    })
                },
                Err(e) => {
                    gloo::console::log!("🔍 DEBUG: AccountSettings request failed:", &e);
                    Err(e)
                },
            };
            link.send_message(Msg::UserLoaded(result));
        });

        Self {
            username: String::new(),
            email: String::new(),
            original_email: String::new(),
            user_id: None,
            csrf_token: None,
            is_loading: true, // Start in loading state
            error_message: None,
            success_message: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::UserLoaded(response) => {
                self.is_loading = false;
                match response {
                    Ok(json_value) => {
                        // Assuming the response structure is UserLoadResponse
                        match serde_json::from_value::<UserLoadResponse>(json_value) {
                            Ok(load_response) => {
                                if load_response.success {
                                    let user = load_response.data.user;
                                    self.username = user.username.clone();
                                    self.email = user.email.clone().unwrap_or_default();
                                    self.original_email = self.email.clone();
                                    self.user_id = Some(user.id.clone());
                                    // CSRF token might be handled by request.rs or from cookie.
                                    // If it's in this response, store it:
                                    self.csrf_token = load_response.data.csrf_token; 
                                    self.error_message = None;
                                } else {
                                    self.error_message = Some("Failed to load user data: API indicated failure.".to_string());
                                }
                            }
                            Err(e) => {
                                self.error_message = Some(format!("Failed to parse user data: {}", e));
                            }
                        }
                    }
                    Err(error_string) => {
                        self.error_message = Some(format!("API Error: {}", error_string));
                    }
                }
                true
            }
            Msg::ProfileUpdateResponse(response) => {
                self.is_loading = false;
                match response {
                    Ok(json_value) => {
                        match serde_json::from_value::<ProfileUpdateResponse>(json_value.clone()) {
                            Ok(update_response) => {
                                if update_response.success {
                                    let updated_user = update_response.data.user;
                                    self.username = updated_user.username.clone();
                                    
                                    let email_was_changed = self.email != self.original_email;
                                    self.email = updated_user.email.clone().unwrap_or_default();
                                    self.original_email = self.email.clone(); // Update original_email to new email

                                    // Update CSRF token if backend sends a new one on update
                                    if update_response.data.csrf_token.is_some() {
                                        self.csrf_token = update_response.data.csrf_token;
                                    }

                                    if email_was_changed && !updated_user.is_email_verified {
                                        self.success_message = Some("Profile updated. A verification email has been sent to your new address. Please check your inbox.".to_string());
                                    } else {
                                        self.success_message = Some(update_response.message);
                                    }
                                    self.error_message = None;

                                    // Clear success message after 5 seconds
                                    let link = ctx.link().clone();
                                    spawn_local(async move {
                                        gloo::timers::future::TimeoutFuture::new(5000).await;
                                        link.send_message(Msg::ClearMessages);
                                    });
                                } else {
                                    // API indicated failure, but successfully parsed response
                                    self.error_message = Some(update_response.message);
                                    self.success_message = None;
                                }
                            }
                            Err(e) => {
                                // Attempt to get a message from the raw JSON if parsing ProfileUpdateResponse fails
                                let message = json_value.get("message").and_then(|m| m.as_str()).unwrap_or("Failed to parse server response.");
                                self.error_message = Some(format!("Error: {}. Details: {}", message, e));
                                self.success_message = None;
                            }
                        }
                    }
                    Err(error_string) => {
                        // Try to parse the error_string as JSON for more specific error
                        if let Ok(json_error) = serde_json::from_str::<serde_json::Value>(&error_string) {
                            let message = json_error.get("message").and_then(|m| m.as_str())
                                .or_else(|| json_error.get("error").and_then(|e| e.as_str()))
                                .unwrap_or("An unknown API error occurred.");
                            self.error_message = Some(message.to_string());
                        } else {
                            self.error_message = Some(format!("API Error: {}", error_string));
                        }
                        self.success_message = None;
                    }
                }
                true
            }
            Msg::UpdateUsername(username) => {
                self.username = username;
                true
            }
            Msg::UpdateEmail(email) => {
                self.email = email;
                true
            }
            Msg::SaveChanges => {
                self.is_loading = true;
                
                // Placeholder for actual API call
                let username = self.username.clone();
                let email = self.email.clone();
                let link = ctx.link().clone();
                
                // Simulate API call with a timeout
                let user_id = self.user_id.clone().unwrap_or_default(); // Get user ID
                let link = ctx.link().clone();
                spawn_local(async move {
                    gloo::console::log!("🔍 DEBUG: AccountSettings updating profile");
                    
                    let update_payload = json!({
                        "username": username,
                        "email": email,
                    });

                    let result = match crate::services::request::RequestInterceptor::put(&format!("/api/cookie/users/{}", user_id))
                        .header("Content-Type", "application/json")
                        .json(&update_payload)
                        .expect("Failed to build request")
                        .send_with_retry()
                        .await {
                        Ok(response) => {
                            gloo::console::log!("🔍 DEBUG: AccountSettings update response received, parsing JSON");
                            response.json().await.map_err(|e| {
                                gloo::console::log!("🔍 DEBUG: Update JSON parse error:", &e.to_string());
                                e.to_string()
                            })
                        },
                        Err(e) => {
                            gloo::console::log!("🔍 DEBUG: AccountSettings update request failed:", &e);
                            Err(e)
                        },
                    };
                    link.send_message(Msg::ProfileUpdateResponse(result));
                });
                
                true
            }
            Msg::ClearMessages => {
                self.error_message = None;
                self.success_message = None;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        
        html! {
            <div class="settings-tab">
                <h3 class="settings-tab__title">{"Account Settings"}</h3>
                
                if self.is_loading {
                    <div class="c-loader">{"Loading..."}</div>
                } else {
                    <div class="settings-tab__content">
                        // Profile Information Card
                        <div class="c-card">
                            <h4 class="c-card__title">{"Profile Information"}</h4>
                            
                            if let Some(message) = &self.success_message {
                                <div class="c-alert c-alert--success">{message}</div>
                            }
                            
                            if let Some(error) = &self.error_message {
                                <div class="c-alert c-alert--error">{error}</div>
                            }
                            
                            <div class="c-form-group">
                                <label class="c-form-label" for="username">{"Username"}</label>
                                <input 
                                    type="text"
                                    id="username"
                                    class="c-form-input"
                                    value={self.username.clone()}
                                    onchange={link.callback(|e: Event| {
                                        let input: HtmlInputElement = e.target_unchecked_into();
                                        Msg::UpdateUsername(input.value())
                                    })}
                                />
                            </div>
                            
                            <div class="c-form-group">
                                <label class="c-form-label" for="email">{"Email Address"}</label>
                                <input 
                                    type="email"
                                    id="email"
                                    class="c-form-input"
                                    value={self.email.clone()}
                                    onchange={link.callback(|e: Event| {
                                        let input: HtmlInputElement = e.target_unchecked_into();
                                        Msg::UpdateEmail(input.value())
                                    })}
                                />
                                <small class="c-form-help">
                                    {"Changing your email will require verification of the new address"}
                                </small>
                            </div>
                            
                            <div class="c-form-actions">
                                <button 
                                    class="c-button c-button--primary"
                                    onclick={link.callback(|_| Msg::SaveChanges)}
                                    disabled={self.is_loading}
                                >
                                    if self.is_loading {
                                        {"Saving..."}
                                    } else {
                                        {"Save Changes"}
                                    }
                                </button>
                            </div>
                        </div>
                        
                        // Account Management Card
                        <div class="c-card c-card--danger">
                            <h4 class="c-card__title">{"Account Management"}</h4>
                            
                            <p>{"Deleting your account will permanently remove all your data. This action cannot be undone."}</p>
                            
                            <div class="c-form-actions">
                                <button class="c-button c-button--danger">
                                    {"Delete Account"}
                                </button>
                            </div>
                        </div>
                    </div>
                }
            </div>
        }
    }
}