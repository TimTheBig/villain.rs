# Villain - Rust Web Framework for WASM

**Villain** is a cutting-edge web framework built in Rust and designed to create highly interactive WebAssembly (WASM) applications. The framework integrates Vue.js templating system, taking the power of Vue's declarative UI and combining it with the speed, safety, and size advantages inherent in Rust and WASM.\
Villain was originally created by **MoeKasp** and **sawmurai**

## Key Features

- **Vue.js Templating**: Leverage Vue's simple and powerful template syntax to create dynamic user interfaces.
- **WASM-Powered**: Experience the high performance, security, and efficiency of Rust-compiled WASM on the web.
- **Rust Ecosystem**: Utilize Rust's robust type safety, memory safety, and concurrency management.
- **Optimized for Speed**: Built for speed, Villain ensures your application is fast and efficient.
- **Small Footprint**: With the compactness of WASM, applications built with Villain have a small binary size, leading to faster load times.
- **Easy to Use**: Villain is designed to be developer-friendly, making it easy to create complex web applications.

## Overview

Villain is designed with the developer's experience in mind. It creates a bridge between the familiar, developer-friendly Vue.js templating system and the Rust-WASM ecosystem. This powerful combination allows you to write high performance, safe, and interactive web applications with less effort and without compromising on the user's experience.

Whether you are a Rustacean looking to venture into the world of WASM or a Vue.js enthusiast wanting to take advantage of Rust's performance and safety, Villain is the perfect tool for you. It brings the best of both worlds under one roof, providing a seamless web development experience.

## Getting Started
First add villain to your `Cargo.toml` file:
```toml
villain = "0.0.1"
futures_signals = "0.3.0"
wasm_bindgen_futures = "0.4.20"
web_sys = "0.3.45"
```
or add it with cargo:
```sh
cargo add villain futures_signals wasm_bindgen_futures web_sys
```

In your project add the `create_entypoint` proc macro to a function:
```rust
use villain::create_entrypoint;

fn main() {
    create_entrypoint!("path/to/your/component.vue");
}
```

To add a Vue component to your project, use the `create_component` macro in the same way.