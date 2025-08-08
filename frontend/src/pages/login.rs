use web_sys::HtmlInputElement;
use yew::prelude::*;
use gloo::net::http::Request;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use crate::services::auth;
use crate::services::auth_context::AuthContext;
use yew_router::prelude::*;
use crate::routes::Route;
use serde_json::Value;

#[derive(Default, Clone, Serialize)]
struct LoginForm {
    username: String,
    password: String,
}

#[derive(Deserialize, Clone, serde::Serialize)]
struct LoginResponse {
    success: bool,
    message: String,
    data: Option<LoginData>,
}

#[derive(Deserialize, Clone, serde::Serialize)]
struct LoginData {
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    refresh_token: String,
    user: User,
}

#[derive(Deserialize, Clone, serde::Serialize)]
struct User {
    id: String,
    username: String,
    email: Option<String>,
    is_email_verified: bool,
    created_at: String,
    #[serde(default)]
    role: String,
    is_active: bool,
    #[serde(default)]
    csrf_token: String,
}

#[function_component(Login)]
pub fn login() -> Html {
    let auth_context = use_context::<AuthContext>().expect("No auth context found");
    let form = use_state(LoginForm::default);
    let error = use_state(|| None::<String>);
    let is_loading = use_state(|| false);
    let navigator = use_navigator().unwrap();
    
    // Check if user is already authenticated and redirect to dashboard
    {
        let auth_context = auth_context.clone();
        let navigator = navigator.clone();
        use_effect_with(auth_context.is_authenticated, move |is_authenticated| {
            if *is_authenticated {
                navigator.push(&Route::Dashboard);
            }
            || {}
        });
    }

    let onsubmit = {
        let form = form.clone();
        let error = error.clone();
        let is_loading = is_loading.clone();
        let navigator = navigator.clone();
        let set_auth = auth_context.set_auth.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let form = form.clone();
            let error = error.clone();
            let is_loading = is_loading.clone();
            let navigator = navigator.clone();
            let set_auth = set_auth.clone();

            is_loading.set(true);
            error.set(None);

            spawn_local(async move {
                let response = Request::post("/users/login")
                    .json(&*form)
                    .expect("Failed to build request")
                    .send()
                    .await;

                is_loading.set(false);

                match response {
                    Ok(resp) => {
                        match resp.json::<LoginResponse>().await {
                            Ok(login_resp) => {
                                let login_resp_clone = login_resp.clone();
                                if login_resp.success {
                                    if let Some(_data) = &login_resp_clone.data {
                                        // Always use the full response processing to ensure proper state reset
                                        let json_value = serde_json::to_value(&login_resp_clone).unwrap_or_default();
                                        auth::store_csrf_token_from_response(&json_value);
                                        
                                        // Set authenticated state immediately to prevent redirect loops
                                        auth::set_authenticated_state(true);
                                        set_auth.emit(true);
                                        navigator.push(&Route::Dashboard);
                                    } else {
                                        error.set(Some("Invalid server response".to_string()));
                                    }
                                } else {
                                    error.set(Some(login_resp.message));
                                }
                            },
                            Err(_) => {
                                // Try to parse as raw JSON to handle potential parsing issues
                                match resp.text().await {
                                    Ok(text) => {
                                        // Try to manually extract CSRF token from JSON
                                        if let Ok(json) = serde_json::from_str::<Value>(&text) {
                                            if let Some(data) = json.get("data") {
                                                if let Some(user) = data.get("user") {
                                                    if let Some(_csrf_token) = user.get("csrf_token").and_then(|t| t.as_str()) {
                                                        // Use the full response processing to ensure proper state reset
                                                        auth::store_csrf_token_from_response(&json);
                                                        
                                                        // Set authenticated state immediately to prevent redirect loops
                                                        auth::set_authenticated_state(true);
                                                        set_auth.emit(true);
                                                        navigator.push(&Route::Dashboard);
                                                        return;
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    Err(_) => {}
                                }
                                
                                error.set(Some("An error occurred while processing the response".to_string()));
                            }
                        }
                    }
                    Err(_) => {
                        error.set(Some("Network error. Please check your connection and try again.".to_string()));
                    }
                }
            });
        })
    };

    let oninput = {
        let form = form.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let mut new_form = (*form).clone();
            match input.name().as_str() {
                "username" => new_form.username = input.value(),
                "password" => new_form.password = input.value(),
                _ => (),
            }
            form.set(new_form);
        })
    };

    html! {
        <div class="auth auth--login">
            <div class="auth__card">
                <div class="auth__header">
                    <h1 class="auth__title">{"Login to Cipher Horizon"}</h1>
                </div>
                <form {onsubmit} class="auth__form">
                <div class="auth__form-group">
                    <label class="auth__label" for="username">{"Username"}</label>
                    <input
                        class="auth__input"
                        type="text"
                        id="username"
                        name="username"
                        value={form.username.clone()}
                        oninput={oninput.clone()}
                        required=true
                    />
                </div>
                <div class="auth__form-group">
                    <label class="auth__label" for="password">{"Password"}</label>
                    <input
                        class="auth__input"
                        type="password"
                        id="password"
                        name="password"
                        value={form.password.clone()}
                        oninput={oninput.clone()}
                        required=true
                    />
                </div>
                <button type="submit" class="auth__button" disabled={*is_loading}>
                    if *is_loading {
                        {"Logging in..."}
                    } else {
                        {"Login"}
                    }
                </button>
                </form>
                if let Some(err) = &*error {
                    <div class="auth__error">{err}</div>
                }
                <div class="auth__links">
                    <Link<Route> to={Route::Register} classes="auth__link">{ "Don't have an account?" }</Link<Route>>
                    <Link<Route> to={Route::PasswordResetRequest} classes="auth__link">{ "Forgot Password?" }</Link<Route>>
                </div>
            </div>
        </div>
    }
}
