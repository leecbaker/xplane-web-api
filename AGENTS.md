# xplane-web-api

## Regeneration

- REST client source is generated at build time from `openapi/xplane-web-api-v3.yaml` via `build.rs`.
- When editing REST behavior, update the OpenAPI file first and rebuild.
- Add Rustdoc where applicable, using text that matches the X-Plane resources as closely as possible.
- When updating `openapi/xplane-web-api-v3.yaml`, ensure that all information on interfaces in the [X-Plane Web API](https://developer.x-plane.com/article/x-plane-web-api/) documentation is included in descriptions.

## Scope

- Keep this crate focused on the X-Plane web APIs.
- REST is generated (Progenitor), while error classification and WebSocket types/client are handwritten.
- CLI support is handwritten, behind the `cli` feature, and disabled by default.
- WebSocket support is behind the `websocket` feature and is disabled by default.
