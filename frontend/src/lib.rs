use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew_router::prelude::*;

mod components;
mod pages;
mod routes;
mod services;
mod confetti;

use components::nav::Nav;
use routes::{Route, switch};
use services::{
    auth_context::AuthContext,
    confetti_context::ConfettiContext,
    ResetTokenContext,
};
use components::footer::Footer;


#[wasm_bindgen(start)]
pub fn run_app() -> Result<(), JsValue> {
    yew::Renderer::<App>::new().render();
    Ok(())
}

#[function_component(App)]
fn app() -> Html {
    let initial_auth_state = services::auth::is_authenticated();
    
    let is_authenticated = use_state(|| initial_auth_state);
    let set_auth = {
        let is_authenticated = is_authenticated.clone();
        Callback::from(move |auth: bool| {
            is_authenticated.set(auth);
        })
    };

    let is_confetti_active = use_state(|| false);
    let set_confetti_active = {
        let is_confetti_active = is_confetti_active.clone();
        Callback::from(move |active: bool| {
            is_confetti_active.set(active);
        })
    };

    let reset_token = use_state(String::default);
    let set_reset_token = {
        let reset_token = reset_token.clone();
        Callback::from(move |token: String| {
            reset_token.set(token);
        })
    };

    let auth_context = AuthContext::new(*is_authenticated, set_auth);
    let confetti_context = ConfettiContext::new(*is_confetti_active, set_confetti_active);
    let reset_token_context = ResetTokenContext::new((*reset_token).clone(), set_reset_token);

    html! {
        <ContextProvider<AuthContext> context={auth_context}>
        <ContextProvider<ConfettiContext> context={confetti_context}>
        <ContextProvider<ResetTokenContext> context={reset_token_context}>
            <BrowserRouter>
                <Nav />
                <Switch<Route> render={switch} />
                <Footer />
            </BrowserRouter>
        </ContextProvider<ResetTokenContext>>
        </ContextProvider<ConfettiContext>>
        </ContextProvider<AuthContext>>
    }
}
