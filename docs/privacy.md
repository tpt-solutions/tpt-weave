# tpt-weave Privacy and Threat Model

## Scope

tpt-weave processes repository source locally and may optionally send reduced
JEv decision requests to a configured provider. The security boundary is the
remote decision request, not the local source tree.

## Protected material

The default privacy policy excludes common environment, credential, private-key,
certificate, and secret-directory paths. `.env`, `.env.*`, `secrets/`,
`credentials/`, `*.pem`, `*.key`, `*.crt`, `*.p12`, and `id_rsa*` patterns are
enabled by default. Repositories may add absolute or relative private path
prefixes through `[privacy].private_paths`.

Source discovery and Rust indexing apply these exclusions before reading or
parsing a file. The policy is local-only by default:
`remote_decisions = false`.

## Redaction

Before an OpenRouter request is built, tpt-weave scans the question, choices,
and state for:

- sensitive environment/configuration assignments;
- password, token, credential, API-key, bearer, and JWT-shaped values;
- PEM certificates and private-key blocks.

Detected values become `[REDACTED]`. Findings contain only type, line, and
column; original values are never stored in redaction metadata or the remote
audit log. Redaction is conservative and intentionally favors false positives
over leaking a credential. It is not a replacement for excluding sensitive
files or reviewing provider configuration.

## Remote audit log

When an audit log is supplied to `OpenRouterProvider`, one content-free JSONL
record is appended before each remote decision request. It contains only:

- repository and provider names;
- item count and estimated token count;
- redaction count;
- a Unix-millisecond timestamp.

An audit write failure fails closed and prevents the remote request.

## Threats and mitigations

| Threat | Mitigation |
| --- | --- |
| Secret in source sent to a provider | Path exclusions, content redaction, local-only default |
| API key stored in configuration | Manifest stores only an environment-variable name |
| Sensitive request retained in telemetry | Audit records are content-free; redaction findings contain no values |
| Stale or malformed privacy configuration | Manifest validation and fail-closed remote-provider construction |
| Accidental traversal through excluded directories | Source walkers skip configured path components before reading |

The scanner is heuristic. Users remain responsible for repository-specific
privacy policy, provider selection, access controls, and reviewing any material
that leaves their machine.
