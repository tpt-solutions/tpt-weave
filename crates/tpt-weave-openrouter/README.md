# tpt-weave-openrouter

Blocking client for the OpenRouter Decisions API (JEv), implementing the
[`tpt-weave-decisions`](../tpt-weave-decisions) `DecisionProvider` trait for
[tpt-weave](https://github.com/tpt-solutions/tpt-weave).

## Wire format

`POST /api/alpha/decisions`

- request: `{ model, state, questions: { answer: { type: "choice", instructions, criteria } } }`
- response: `{ answers: { answer: { type: "choice", choice, confidence, probabilities } }, usage: { input_tokens, output_tokens, cost } }`

## What it provides

- `OpenRouterProvider` - configured from `JevConfig` / `ProviderConfig` in the manifest. Handles timeouts and
  retries (transport failures, HTTP 429 and 5xx); API-key and validation errors fail immediately.
- `ParsedAnswer` - choice, confidence (defaulting to `DEFAULT_CONFIDENCE` when omitted), token usage, USD cost,
  response id and the served model snapshot.
- `RemoteAuditLog`, `RemoteAuditEntry` - content-free audit trail (`AUDIT_FILE`,
  `.tpt-weave/privacy-audit.jsonl`) of every remote call.
- Constants: `QUESTION_KEY`, `APP_TITLE`, `DEFAULT_CONFIDENCE`.

## Privacy

Provider construction **fails closed**: with a local-only privacy configuration no client is built and no
network traffic is possible. Secrets are redacted before any request leaves the process. The audit log records
that a call happened, never its content. See [docs/privacy.md](../../docs/privacy.md) and
[docs/openrouter.md](../../docs/openrouter.md).

## Design notes

- `#![forbid(unsafe_code)]`; TLS via `rustls` (no OpenSSL).
- Blocking by design - decisions are small and sequenced inside deterministic pipelines.

## Status

Part of the [tpt-weave](https://github.com/tpt-solutions/tpt-weave) workspace (pre-alpha, version 0.1.0,
MSRV Rust 1.85, edition 2024). The crate is not yet published to crates.io (`publish = false`; see
[docs/release-readiness.md](../../docs/release-readiness.md)). Public APIs are stable-by-intent unless noted;
experimental APIs are gated behind the `unstable` feature where present (see
[docs/decisions.md](../../docs/decisions.md) section 6). Changes are tracked in [CHANGELOG.md](CHANGELOG.md).

## Related documentation

- [Architecture](../../docs/architecture.md) - crate layering and data flow
- [Design specification](../../spec.md)
- [Using tpt-weave](../../docs/using-tpt-weave.md)

## License

MIT - see [LICENSE](../../LICENSE).
