use yew::prelude::*;
use yew_router::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::pages::{
    Home, About, Login, Dashboard, Register, EmailVerified, RegistrationComplete,
    PasswordResetRequest, PasswordResetVerify, PasswordResetNew
};
use crate::services::auth::{is_authenticated, validate_session};

#[derive(Clone, Routable, PartialEq, Debug)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/about")]
    About,
    #[at("/login")]
    Login,
    #[at("/dashboard")]
    Dashboard,
    #[at("/register")]
    Register,
    #[at("/email_verified")]
    EmailVerified,
    #[at("/registration_complete")]
    RegistrationComplete,
    #[at("/password-reset")]
    PasswordResetRequest,
    #[at("/password-reset/verify")]
    PasswordResetVerify,
    #[at("/password-reset/new")]
    PasswordResetNew,  // Token is handled through context instead of route params
    #[not_found]
    #[at("/404")]
    NotFound,
}

#[derive(Properties, PartialEq)]
struct ProtectedRouteProps {
    pub children: Children,
}

#[function_component(ProtectedRoute)]
fn protected_route(props: &ProtectedRouteProps) -> Html {
    let validation_state = use_state(|| None::<bool>);
    let is_validating = use_state(|| false);

    let validation_state_clone = validation_state.clone();
    let is_validating_clone = is_validating.clone();

    // Perform session validation when component mounts
    use_effect_with((), move |_| {
        let validation_state = validation_state_clone.clone();
        let is_validating = is_validating_clone.clone();

        // Always start validation if we don't have a cached result
        if *validation_state == None && !*is_validating {
            is_validating.set(true);
            spawn_local(async move {
                let is_valid = validate_session().await;
                validation_state.set(Some(is_valid));
                is_validating.set(false);
            });
        }
        || {}
    });

    match (*validation_state, *is_validating) {
        (Some(true), _) => {
            // Session is valid, render protected content
            html! { <>{ for props.children.iter() }</> }
        }
        (Some(false), _) => {
            // Session is invalid, redirect to login
            html! {
                <div class="auth-redirect">
                    <p>{"Session expired. Please log in again."}</p>
                    <script>
                        {"window.location.href = '/login';"}
                    </script>
                </div>
            }
        }
        (None, true) => {
            // Still validating, show loading
            html! {
                <div class="auth-validating">
                    <p>{"Validating session..."}</p>
                </div>
            }
        }
        (None, false) => {
            // Don't redirect immediately - show loading while validation starts
            html! {
                <div class="auth-validating">
                    <p>{"Checking authentication..."}</p>
                </div>
            }
        }
    }
}

pub fn switch(routes: Route) -> Html {
    match routes {
        Route::Home => html! { <Home /> },
        Route::About => html! { <About /> },
        Route::Login => {
            // Simply render the login page - let the login component handle authentication checks
            html! { <Login /> }
        },
        Route::Dashboard => {
            html! {
                <ProtectedRoute>
                    <Dashboard />
                </ProtectedRoute>
            }
        },
        Route::Register => {
            // Simply render the register page - let the register component handle authentication checks
            html! { <Register /> }
        },
        Route::EmailVerified => html! { <EmailVerified /> },
        Route::RegistrationComplete => html! { <RegistrationComplete /> },
        Route::PasswordResetRequest => html! { <PasswordResetRequest /> },
        Route::PasswordResetVerify => html! { <PasswordResetVerify /> },
        Route::PasswordResetNew => html! { <PasswordResetNew /> },
        Route::NotFound => html! { <h1>{"404 Not Found"}</h1> },
    }
}
