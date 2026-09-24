# tpt-weave Security Review

Review date: 2026-09-24  
Scope: repository-local security/privacy review of the current working tree.

## Findings

| Area | Result | Evidence / limitation |
| --- | --- | --- |
| Unsafe code | Pass | Workspace crates use `#![forbid(unsafe_code)]`; no unsafe implementation was added. |
| Secret storage | Pass | Provider configuration stores an environment-variable name, not a key. Source scan found only test/example secret-shaped literals. |
| Remote transport | Pass with configuration risk | OpenRouter is the only production HTTP client. TLS uses reqwest `rustls-tls`; endpoint schemes and request shape are validated. A user can explicitly configure another `http(s)` endpoint. |
| Local-only default | Pass | `remote_decisions` defaults to false; privacy-aware provider construction fails closed when remote decisions are disabled. |
| Redaction | Pass with stated heuristic limits | Assignments, common provider-token prefixes, bearer/JWT forms, certificates, and private-key blocks are replaced. Unknown secret formats require repository-specific exclusions and review. |
| Audit privacy | Pass | Audit records contain only provider/repository identifiers, counts, token estimate, redaction count, and timestamp. Audit write failure prevents transmission. |
| Filesystem privacy | Pass with pattern limitation | Default secret paths and private prefixes are filtered before reads; source walkers skip symlink entries. Glob-lite matching supports documented forms, not arbitrary regular expressions. |
| Filesystem cache | Pass | Cache envelopes verify key/schema, reject corrupt entries, and remove foreign revisions/schema data. `.tpt-weave/` is ignored by default. |
| Error messages | Pass | Remote error bodies are truncated; secrets are not intentionally copied into logs. Provider keys and full request bodies are not serialized. |
| Dependencies | Release action required | Run `cargo audit` or an approved dependency scanner in release CI. No live dependency advisory service was consulted for this local review. |

## Required operational policy

1. Keep `remote_decisions = false` unless remote transfer is approved.
2. Set the API key only in the configured environment variable.
3. Review `[privacy].exclude` and `private_paths` for repository-specific secrets.
4. Treat redaction as defense in depth, not proof that arbitrary credentials are detected.
5. Run the provider against loopback scripted tests before enabling it in an agent environment.

## Release disposition

The implementation-level review is complete. A release still requires an independent dependency audit and review of any new downstream configuration or provider policy. Publication and external security sign-off remain open in `todo.md`.
