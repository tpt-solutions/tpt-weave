# tpt-weave JEv Guide

## Model boundary

JEv is an optional decision component, not the context transport. Deterministic
ranking and representation happen first; a provider answers only a bounded,
structured choice.

## Configuration

The manifest contains model, confidence threshold, timeout, and retry settings.
The provider manifest stores an environment-variable name, never an API key.
The default privacy mode is local-only:

```toml
[jev]
enabled = true
model = "typesafe/jev-1.13"
min_confidence = 0.70
timeout_ms = 5000
max_retries = 2

[privacy]
remote_decisions = false
```

Enable remote transfer only after reviewing exclusions, redaction behavior, the
provider, and budget. Remote requests redact questions, choices, and state;
audit records are metadata-only.

## Failure semantics

Provider transport failures, invalid choices, and low confidence are handled by
the policy engine. The engine records a deterministic fallback or a safety
override where configured. Under `--strict-decisions`, CLI callers can request a
hard failure instead of silently degrading.
