# X-Plane Web APIs

Typed Rust access to the documented X-Plane local web APIs.

## Contents

- REST client generated at build time from `openapi/xplane-web-api-v3.yaml` using Progenitor.
- Shared REST error classification (`xplane_web_api::error::RestClientError`).
- Optional `xplane-web-api` CLI for REST operations.
- Optional typed WebSocket request/response models and an async convenience client.

## Features

- Default features: none (REST only).
- `cli`: enables the `xplane-web-api` command-line tool.
- `websocket`: enables typed websocket models + async websocket client.

Run the CLI:

```bash
cargo run -p xplane-web-api --features cli --bin xplane-web-api -- --help
```

CLI logging uses `env_logger`; control verbosity with `RUST_LOG`:

```bash
RUST_LOG=debug cargo run -p xplane-web-api --features cli --bin xplane-web-api -- --help
```

Enable websocket support as a library dependency:

```bash
cargo add xplane-web-api --features websocket
```

## Source docs

The API model in this crate is based on:

- <https://developer.x-plane.com/article/x-plane-web-api/>
