# tpt-weave Token Accounting

## Core definition

The deterministic estimate is one token per four Unicode scalar values, rounded
up:

```text
estimated_tokens(text) = ceil(character_count / 4)
```

It is an estimate, not a provider tokenizer. Keep the same function when
comparing baseline and optimized runs so the measurement is internally
consistent.

## Context accounting

A context response records per-candidate estimates plus aggregate selected,
included, omitted, and budget information. The primary comparison is:

```text
gross reduction = 1 - delivered / raw
net reduction   = 1 - (delivered + jev input) / raw
```

JEv output tokens and provider cost are reported separately when available.
Cache savings are a secondary metric and must not be double-counted as
delivered-token reduction.

## Measurement discipline

Record the same task, budget, representation level, provider model, and capture
format for both sides. Store token counts and metadata, not prompts or source
content. See [experiments.md](experiments.md) and [workload.md](workload.md).
