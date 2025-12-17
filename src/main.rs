mod guide_component;

use dioxus::prelude::*;
use guide_component::Chapter3;
use tracing::Level;

// Only needed on the web/wasm target for talking to `window.ethereum`
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use wasm_bindgen_futures::{spawn_local, JsFuture};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use web_sys::Window;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use js_sys::{Array, Function, Object, Reflect};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const HEADER_SVG: Asset = asset!("/assets/header.svg");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

/// Server entrypoint when built with `--features server`
#[cfg(feature = "server")]
fn main() {
    let _ = dioxus::logger::init(Level::DEBUG);
    tracing::debug!("Starting fullstack server!");

    dioxus::serve(|| async move {
        // Build an Axum router that serves the fullstack Dioxus app
        let router = dioxus::server::router(App);
        Ok(router)
    });
}

/// Client-only entrypoint (web, desktop, etc.) when *not* built with `server`
#[cfg(not(feature = "server"))]
fn main() {
    let _ = dioxus::logger::init(Level::DEBUG);
    tracing::debug!("Rendering app!");
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        div {
            class: "min-h-screen bg-slate-900 text-slate-100 flex justify-center",
            div {
                class: "w-full max-w-6xl flex flex-col gap-10 p-4 md:p-6",

                // Full-width hero at the top
                Hero {}

                // 3-column content grid below the hero
                div {
                    class: "grid grid-cols-1 md:grid-cols-3 gap-8",

                    // Left column (placeholder or future content)
                    div {
                        class: "space-y-4",
                    }

                    // Middle column (currently empty, hero holds signup)
                    div {
                        class: "space-y-4",
                    }

                    // Right column
                    div {
                        class: "space-y-4",
                        Chapter3 {}
                    }
                }
            }
        }
    }
}

#[component]
pub fn Hero() -> Element {
    let mut status = use_signal(|| "Not connected".to_string());
    let mut address = use_signal(|| None::<String>);
    let mut eth_balance = use_signal(|| None::<String>);

    rsx! {
        div {
            id: "hero",
            class: "flex items-center justify-between gap-4 rounded-2xl bg-slate-900/70 border border-slate-700/60 px-4 py-3 md:px-6 md:py-4 backdrop-blur",
            div {
                class: "flex items-center gap-3",
                img { src: HEADER_SVG, id: "header", class: "h-9 w-auto md:h-10 drop-shadow-[0_0_16px_rgba(168,85,247,0.55)]" }
                div {
                    class: "flex flex-col",
                    span {
                        class: "text-sm font-semibold uppercase tracking-[0.16em] text-purple-300",
                        "Dioxus FFT"
                    }
                    span {
                        class: "text-xs md:text-sm text-slate-300",
                        "Fullstack Dioxus 0.7 · Web3 signup · Dog viewer"
                    }
                }
            }
            div {
                class: "flex items-center gap-3 text-xs md:text-sm",
                span {
                    class: "hidden md:inline text-slate-300",
                    if let Some(addr) = address() {
                        "Connected: {addr}"
                    } else {
                        "{status}"
                    }
                }
                if let Some(balance) = eth_balance() {
                    span {
                        class: "hidden md:inline text-slate-400",
                        "· ETH: {balance}"
                    }
                }
                button {
                    class: "inline-flex items-center justify-center px-4 py-2 rounded-xl bg-gradient-to-r from-violet-500 via-purple-500 to-fuchsia-500 \
                            text-white font-semibold shadow-md shadow-purple-500/40 hover:from-violet-400 hover:via-purple-400 hover:to-fuchsia-400 \
                            focus:outline-none focus:ring-2 focus:ring-purple-400 focus:ring-offset-2 focus:ring-offset-slate-900 \
                            text-xs md:text-sm transition-colors duration-200",
                    onclick: move |_| {
                        // Pure Rust+wasm connector that calls `window.ethereum.request({ method: "eth_requestAccounts" })`
                        #[cfg(all(feature = "web", target_arch = "wasm32"))]
                        {
                            let mut status = status.to_owned();
                            let mut address = address.to_owned();
                            let mut eth_balance = eth_balance.to_owned();
                            spawn_local(async move {
                                *status.write() = "Connecting...".to_string();
                                match connect_with_injected_wallet().await {
                                    Ok(Some(addr)) => {
                                        *status.write() = "Connected".to_string();
                                        let addr_clone = addr.clone();
                                        *address.write() = Some(addr_clone.clone());

                                        // Fetch ETH balance for the connected address
                                        match get_eth_balance(&addr_clone).await {
                                            Ok(bal) => {
                                                *eth_balance.write() = Some(bal);
                                            }
                                            Err(e) => {
                                                *status.write() = format!("Balance error: {e}");
                                            }
                                        }
                                    }
                                    Ok(None) => {
                                        *status.write() = "No injected wallet found".to_string();
                                    }
                                    Err(e) => {
                                        *status.write() = format!("Error: {e}");
                                    }
                                }
                            });
                        }

                        #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
                        {
                            *status.write() = "Wallet connect is only available on the web".to_string();
                        }
                    },
                    "Sign up"
                }
            }
        }
    }
}

/// Call `window.ethereum.request({ method: "eth_requestAccounts" })` and return the first address, if any.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
async fn connect_with_injected_wallet() -> Result<Option<String>, String> {
    // Get `window`
    let window: Window = web_sys::window().ok_or("No window object")?;

    // Read `window.ethereum`
    let ethereum = Reflect::get(&JsValue::from(window), &JsValue::from_str("ethereum"))
        .map_err(|_| "Failed to read window.ethereum")?;

    if ethereum.is_undefined() || ethereum.is_null() {
        return Ok(None);
    }

    // Build params: { method: "eth_requestAccounts" }
    let params = Object::new();
    Reflect::set(
        &params,
        &JsValue::from_str("method"),
        &JsValue::from_str("eth_requestAccounts"),
    )
    .map_err(|_| "Failed to set request method")?;

    // Get ethereum.request
    let request_val = Reflect::get(&ethereum, &JsValue::from_str("request"))
        .map_err(|_| "Failed to get ethereum.request")?;
    let request_fn: Function = request_val
        .dyn_into()
        .map_err(|_| "ethereum.request is not a function")?;

    // Call request and await the Promise
    let promise_val = request_fn
        .call1(&ethereum, &params)
        .map_err(|_| "Failed to call ethereum.request")?;
    let promise = promise_val
        .dyn_into::<js_sys::Promise>()
        .map_err(|_| "request did not return a Promise")?;

    let result: JsValue = JsFuture::from(promise)
        .await
        .map_err(|e| format!("request rejected: {:?}", e))?;

    // Result should be an array of addresses; take the first.
    let accounts: Array = result
        .dyn_into()
        .map_err(|_| "Unexpected response from wallet")?;

    let first = accounts.get(0);
    if first.is_undefined() || first.is_null() {
        return Err("No accounts returned from wallet".into());
    }

    Ok(Some(first.as_string().unwrap_or_default()))
}

/// Fetch the ETH balance for the given address using `eth_getBalance` and format it in ETH.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
async fn get_eth_balance(address: &str) -> Result<String, String> {
    // Get `window`
    let window: Window = web_sys::window().ok_or("No window object")?;

    // Read `window.ethereum`
    let ethereum = Reflect::get(&JsValue::from(window), &JsValue::from_str("ethereum"))
        .map_err(|_| "Failed to read window.ethereum")?;

    if ethereum.is_undefined() || ethereum.is_null() {
        return Err("No injected wallet found".into());
    }

    // Build request: { method: "eth_getBalance", params: [address, "latest"] }
    let request = Object::new();
    let params_array = Array::new();
    params_array.push(&JsValue::from_str(address));
    params_array.push(&JsValue::from_str("latest"));

    Reflect::set(
        &request,
        &JsValue::from_str("method"),
        &JsValue::from_str("eth_getBalance"),
    )
    .map_err(|_| "Failed to set method")?;
    Reflect::set(
        &request,
        &JsValue::from_str("params"),
        &params_array,
    )
    .map_err(|_| "Failed to set params")?;

    // Get ethereum.request
    let request_val = Reflect::get(&ethereum, &JsValue::from_str("request"))
        .map_err(|_| "Failed to get ethereum.request")?;
    let request_fn: Function = request_val
        .dyn_into()
        .map_err(|_| "ethereum.request is not a function")?;

    // Call request and await the Promise
    let promise_val = request_fn
        .call1(&ethereum, &request)
        .map_err(|_| "Failed to call ethereum.request")?;
    let promise = promise_val
        .dyn_into::<js_sys::Promise>()
        .map_err(|_| "eth_getBalance did not return a Promise")?;

    let result: JsValue = JsFuture::from(promise)
        .await
        .map_err(|e| format!("eth_getBalance rejected: {:?}", e))?;

    // Result should be a hex string like "0x1234..."
    let balance_hex = result
        .as_string()
        .ok_or("Unexpected balance response")?;

    // Strip "0x" prefix and parse as u128 wei
    let cleaned = balance_hex.trim_start_matches("0x");
    let wei = u128::from_str_radix(cleaned, 16).map_err(|_| "Failed to parse balance")?;

    // Convert to ETH as a simple decimal string with 4 decimal places
    let eth = wei as f64 / 1e18_f64;
    Ok(format!("{:.4}", eth))
}
