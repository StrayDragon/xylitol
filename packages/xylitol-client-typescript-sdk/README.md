# xylitol-client-typescript-sdk

TypeScript bindings for the product four-quadrant envelope + method table.

- `bindings.ts` is **generated** by `just gen-sdk` (specta export from the
  Rust protocol types). DO NOT EDIT by hand.
- Drift between the generated output and this checked-in copy fails `just qa`
  (`scripts/check_protocol_ts_bindings.py`).
- OpenAPI / AsyncAPI / salvo oapi are **not** the type SSOT (spec pa-bind1).
- Status: types only. The thin fetch/WS client (unary / respond / mux +
  writerToken lease) is drafted as change `c2310-add-web-ts-client` and lands
  here when a Web surface gets a real green light.
