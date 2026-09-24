# tpt-weave OpenRouter Guide

The OpenRouter adapter is a blocking HTTP client for the Decisions API. It
retries transport failures and retryable status codes, fails immediately for
configuration and client errors, and reports latency, confidence, input/output
tokens, and cost when the provider supplies them.

Set the environment variable named by `[provider].api_key_env` before using a
remote provider. Do not place keys in the manifest, command arguments, source
files, or workload captures. Test with a local scripted HTTP server; the test
suite does not call the live API.

The provider is local-only by default. `remote_decisions = true` is an explicit
opt-in, and `OpenRouterProvider::new_with_privacy` rejects a disabled policy.
See [privacy.md](privacy.md) before enabling it.
