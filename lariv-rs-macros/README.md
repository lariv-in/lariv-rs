# lariv-rs-macros

Procedural macros for [`lariv-rs`](https://crates.io/crates/lariv-rs), a compile-time plugin web application framework.

## Macros

- `html_form` — serde field wiring and `HtmlForm` trait implementation
- `define_plugin_routes` — route tags, URL builders, response traits, and `RouteRegistrar` hook
- `main` — large-stack async `main` for deep HList install/mount

This crate is typically used through `lariv-rs` rather than directly.

## License

MIT
