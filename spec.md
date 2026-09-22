tpt-weave — Design Specification

Status: Proposed
Language: Rust
License: MIT
Repository: tpt-weave
Purpose: Cross-repository AI context optimisation for the TPT Solutions ecosystem.

1. Executive Summary

tpt-weave is the reusable AI-context infrastructure layer for TPT Solutions repositories.

Its purpose is to reduce the amount of information that expensive coding models such as GLM-5.3-Flash must receive while preserving the information required to complete a task correctly.

The core principle is:

Do not compress information merely because it is large. Determine what information is necessary, expose the smallest useful representation first, and retrieve more detail only when required.

tpt-weave combines:

deterministic repository indexing;

Rust AST and symbol extraction;

Cargo dependency and workspace analysis;

cross-repository dependency knowledge;

hierarchical source representations;

context retrieval;

deterministic tool-output reduction;

caching;

token accounting;

JEv-based context decisions;

OpenAI-compatible/MCP integration;

optional integration with existing TPT AI infrastructure.

The system should work without modifying application source code. A repository should become AI-efficient primarily by adding generated metadata and connecting the coding agent to tpt-weave.

2. Why This Exists

TPT development now spans hundreds of repositories and a growing family of reusable Rust primitives.

Coding agents repeatedly spend tokens discovering:

repository structure;

Cargo workspaces;

dependency relationships;

module boundaries;

public APIs;

symbol definitions;

implementation details;

previous tool results;

test results;

compiler diagnostics;

documentation;

unrelated files.

At very high agent throughput, unnecessary context becomes a direct operating cost.

The target is therefore not simply "better prompts".

The target is a reusable context infrastructure:

User task
    |
    v
tpt-weave
    |
    +--> understand repository
    +--> understand dependency graph
    +--> determine relevance
    +--> expose compact representation
    +--> retrieve details on demand
    +--> cache repeated context
    |
    v
Expensive coding model

3. Design Goals

3.1 Primary goals

Reduce model input tokens without sacrificing correctness.

Reduce repeated repository exploration.

Make all TPT repositories machine-readable.

Work across unrelated domains.

Work with GLM-5.3-Flash and other OpenAI-compatible models.

Use JEv for structured decisions where appropriate.

Prefer deterministic algorithms over LLM calls whenever possible.

Preserve exact source locally; never make lossy compression the only representation.

Support incremental updates after source changes.

Provide measurable token savings.

Remain useful even when JEv or a remote LLM is unavailable.

Keep application repositories free of unnecessary AI-specific source-code coupling.

3.2 Secondary goals

Support external users adopting individual TPT crates.

Provide an MCP interface.

Provide a CLI.

Support future local models.

Support future TPT agents.

Become the standard AI-context format for TPT repositories.

3.3 Non-goals

tpt-weave is not:

a general-purpose LLM;

a replacement for Git;

a replacement for Cargo;

a source-code minifier;

a vector database requirement;

an autonomous coding agent;

a system that rewrites source code to save tokens;

dependent on JEv for basic operation.

4. JEv Integration

OpenRouter currently exposes typesafe/jev-1.13 as a structured decision model.

Current listed pricing is:

input: $0.042 / 1M tokens;

output: $0 / 1M tokens;

context: 32K tokens.

OpenRouter exposes Jev through its Decisions API rather than the normal chat-completions path.

Reference:

OpenRouter model: https://openrouter.ai/typesafe/jev-1.13

OpenRouter Decisions API: POST https://openrouter.ai/api/alpha/decisions

4.1 Important architectural rule

JEv should not be responsible for all repository understanding.

tpt-weave should first perform deterministic filtering.

JEv then makes small, structured decisions over the reduced candidate set.

Example:

Task:
"Fix the image resampling bug."

Deterministic index:
    tpt-cv
    image module
    resample module
    PixelBuffer
    Filter
    tpt-math dependency

JEv:
    relevant module?       yes
    relevant dependency?   yes
    tests relevant?        yes
    documentation relevant? no
    expand implementation? yes

This prevents JEv itself becoming another expensive context consumer.

4.2 JEv decision categories

Initial decision schemas:

Relevance

candidate -> relevant / irrelevant

Expansion

representation -> keep / expand / defer

Dependency traversal

dependency -> traverse / do-not-traverse

Tool-result retention

tool result -> retain / summarize / discard

Context eviction

context item -> keep / cache / evict

Task classification

task -> bugfix / feature / refactor / docs / test / build / exploration

Retrieval depth

none / metadata / symbols / signatures / skeleton / implementation / full-file

4.3 Confidence

Jev probabilities should not be treated as proof.

For each decision:

decision
confidence
policy threshold
action

Thresholds must be configurable and evaluated against real TPT tasks.

Safety-sensitive or correctness-critical actions should use conservative thresholds and deterministic checks.

5. Core Architecture

                    ┌──────────────────────┐
                    │ Coding Agent         │
                    │ GLM / Claude / etc.  │
                    └──────────┬───────────┘
                               │
                               v
                    ┌──────────────────────┐
                    │ tpt-weave Gateway    │
                    └──────────┬───────────┘
                               │
              ┌────────────────┼────────────────┐
              v                v                v
       ┌─────────────┐  ┌──────────────┐  ┌────────────┐
       │ Repo Index  │  │ JEv Decision │  │ Cache      │
       └──────┬──────┘  └──────┬───────┘  └─────┬──────┘
              │                │                │
              └────────────────┼────────────────┘
                               v
                    ┌──────────────────────┐
                    │ Context Builder      │
                    └──────────┬───────────┘
                               v
                    ┌──────────────────────┐
                    │ Selected Context     │
                    └──────────────────────┘

6. Repository Representation

Every indexed repository receives a generated .tpt-weave/ directory.

Suggested layout:

.tpt-weave/
├── manifest.toml
├── repository.json
├── symbols.bin
├── symbols.json
├── dependencies.json
├── files.json
├── api.json
├── architecture.md
├── summaries/
├── skeletons/
├── tool-policies.toml
└── cache/

Generated files should normally be ignored by Git for private working state.

A project may optionally commit a small deterministic manifest if desired.

7. Repository Manifest

Example:

schema = 1
repository = "tpt-cv"
language = "rust"
workspace = true

[features]
symbol_index = true
dependency_graph = true
source_skeletons = true
tool_output_reduction = true
jev = true

[context]
default_budget = 12000
maximum_budget = 32000

The manifest is configuration, not generated prose.

8. Deterministic Repository Index

The indexer should use deterministic information first.

8.1 Cargo

Use:

cargo metadata

to obtain:

packages;

crates;

workspace members;

dependencies;

targets;

features;

versions;

paths.

8.2 Rust syntax

Use syn to extract:

modules;

structs;

enums;

traits;

impl blocks;

functions;

methods;

constants;

type aliases;

macros;

attributes;

visibility;

signatures.

8.3 Git

Use Git metadata for:

changed files;

recent commits;

blame;

file history;

branch state;

current diff.

Git history should be retrieved only when relevant.

8.4 Compiler information

Where practical, support:

rustdoc JSON;

compiler diagnostics;

type information;

symbol references.

9. Hierarchical Context Levels

Every source artifact should have progressively more detailed representations.

Level 0 — Metadata

resample.rs
module=image
LOC=412
exports=4
dependencies=3

Level 1 — Symbols

resize()
resample()
Resampler
Filter

Level 2 — Signatures

pub fn resize(
    image: &Image,
    width: usize,
    height: usize,
    filter: Filter
) -> Result<Image>

Level 3 — Skeleton

Structure remains; implementation bodies are omitted.

Level 4 — Relevant implementation

Only requested/relevant functions and supporting definitions.

Level 5 — Complete source

Full file or complete related module.

The agent should normally start at the lowest level that can answer its current question.

10. Retrieval Strategy

Retrieval order:

task
  |
  v
repository
  |
  v
crate
  |
  v
module
  |
  v
symbol
  |
  v
dependency
  |
  v
implementation

Avoid global source search until cheaper indexes fail.

Example

Task:

"Fix the resampling bug in tpt-cv."

Initial context:

tpt-cv
image
resample
PixelBuffer
Filter
related tests

If the agent requests PixelBuffer::view_mut, retrieve only that symbol and its relevant implementation.

11. Cross-Repository Graph

tpt-weave must understand TPT repositories as a graph.

Example:

tpt-math
   |
   +--> tpt-cv
           |
           +--> tpt-teleop-agri

And:

tpt-media
   |
   +--> tpt-kinetix
   +--> tpt-cadence
   +--> tpt-visual
   +--> tpt-audio
   +--> tpt-voice

The graph should record:

repository;

package;

crate;

module;

symbol;

dependency;

version;

feature;

API relationship.

Cross-repository traversal should be opt-in or JEv-selected.

12. Existing TPT Repository Integration

tpt-weave should be a separate generic infrastructure repository.

Existing repositories should not be rewritten to depend on it merely to function.

Instead use three integration levels.

Level A — Zero-code integration

Install the tpt-weave CLI and index the repository:

tpt-weave init
tpt-weave index

No source-code changes.

Level B — Agent integration

Configure the coding agent/MCP client to use:

tpt-weave context
tpt-weave search
tpt-weave symbol
tpt-weave expand
tpt-weave diff
tpt-weave test-result

Level C — Native Rust integration

For repositories that need programmatic access:

[dev-dependencies]
tpt-weave = "..."

or the appropriate split crate once the API stabilises.

Do not force Level C on ordinary libraries.

13. Recommended TPT Integration Order

Start with repositories where context complexity is high and changes are frequent.

Phase 1

tpt-code-command-center

tpt-infer

tpt-uir

tpt-raglite

Phase 2

tpt-cv

tpt-teleop-agri

tpt-math

tpt-sensetel

Phase 3

tpt-kinetix

tpt-cadence

tpt-visual

tpt-audio

tpt-voice

tpt-av-*

Phase 4

Broad TPT rollout.

tpt-raglite should be evaluated as an optional retrieval/storage component rather than duplicated inside tpt-weave.

tpt-uir should be evaluated as a possible transport/IR boundary where it provides a useful stable representation.

tpt-sensetel should remain focused on semantic capture. Its decision/compression concepts may inform tpt-weave, but the repositories should not be conflated.

14. Tool Output Optimisation

Tool output can be a major source of waste.

tpt-weave should transform verbose tool output into compact deterministic records.

Example:

cargo test
184 passed
0 failed
2 ignored

instead of retaining thousands of lines when there are no failures.

If failures occur:

TEST FAILURE
tests: 184
passed: 182
failed: 2

FAILURES:
crate::foo::test_a
crate::bar::test_b

DETAIL AVAILABLE: yes

Full raw output remains locally available.

The model can request it.

Apply the same approach to:

cargo check;

cargo test;

cargo clippy;

cargo build;

git diff;

git status;

directory listings;

grep/search;

JSON tool results;

logs;

compiler diagnostics.

15. Cache Architecture

Separate:

Stable context

repository manifest;

architecture;

symbol index;

dependency graph;

tool schemas.

Dynamic context

user request;

current diff;

compiler output;

test failures;

recent edits.

Stable context should be reused whenever possible.

Cache keys should include:

repository revision
index schema
query
context policy
representation level

Never reuse stale source representations after a relevant source change.

16. Token Accounting

Every context operation should record:

raw_tokens
selected_tokens
saved_tokens
selection_ratio
jev_tokens
model_tokens
cache_hit

Example:

{
  "raw_tokens": 48231,
  "selected_tokens": 7312,
  "saved_tokens": 40919,
  "selection_ratio": 0.1516,
  "jev_tokens": 612,
  "cache_hit": true
}

The system should report:

Raw context:       48,231
Delivered:          7,312
Reduction:             84.8%
JEv overhead:          612
Net reduction:        83.5%

The goal is measurable net savings, not theoretical compression.

17. JEv Cost Model

At the currently listed OpenRouter price of $0.042/M input tokens and free output, JEv is cheap enough to use as a frequent decision layer.

However, JEv should not receive the entire repository.

Correct:

deterministic retrieval
    -> 5,000 candidate tokens
    -> JEv
    -> 1,000 useful tokens

Incorrect:

entire 500,000-token repository
    -> JEv

The deterministic index must do the first reduction.

18. OpenRouter Adapter

Implement a provider-neutral interface:

pub trait DecisionProvider {
    async fn decide(
        &self,
        request: DecisionRequest
    ) -> Result<DecisionResponse>;
}

Initial provider:

OpenRouter
    model = typesafe/jev-1.13
    endpoint = /api/alpha/decisions

Future providers:

TypeSafe direct
local decision model
mock provider
offline deterministic provider

The application should not be coupled directly to OpenRouter.

19. MCP Interface

Expose context operations through MCP.

Suggested tools:

tpt_repo_overview
tpt_find_symbol
tpt_find_references
tpt_get_signature
tpt_get_skeleton
tpt_expand
tpt_dependencies
tpt_related
tpt_git_diff
tpt_test_result
tpt_context_stats

Tool schemas themselves should be kept compact.

tpt-weave should expose only the tools relevant to the current task where practical.

20. Agent Workflow

Initial request

User
  |
  v
Classify task
  |
  v
Identify repository
  |
  v
Build candidate context
  |
  v
JEv relevance decisions
  |
  v
Build minimal context
  |
  v
GLM

During work

GLM requests more information
  |
  v
tpt-weave
  |
  +--> cache?
  +--> deterministic index?
  +--> JEv?
  |
  v
smallest useful response

After tool execution

tool output
  |
  v
deterministic reducer
  |
  v
retain important facts
  |
  v
raw output remains locally available

21. Correctness Model

tpt-weave must optimise context without silently changing facts.

Therefore:

Source code remains authoritative.

Generated indexes are derived artifacts.

Summaries are never the only representation.

Skeletons never replace source.

JEv decisions are advisory unless a deterministic policy says otherwise.

Any uncertainty can trigger context expansion.

The model can request full source.

Cache invalidation must be conservative.

A key design principle:

Lossless retrieval is preferred to lossy summarisation.

22. Failure Modes

JEv unavailable

Fall back to deterministic relevance ranking.

OpenRouter unavailable

Use configured secondary decision provider.

Index stale

Re-index automatically.

Decision confidence low

Expand context.

Agent asks for unknown symbol

Run deterministic lookup.

Context budget exceeded

Evict low-value dynamic context first.

Wrong relevance decision

Allow explicit expansion and record the miss for evaluation.

23. Evaluation

Create a benchmark corpus from real TPT tasks.

For each task measure:

raw context tokens;

delivered tokens;

output tokens;

total tokens;

JEv tokens;

latency;

cost;

task success;

number of retrieval calls;

context misses;

unnecessary expansions.

Primary metric:

Net Token Reduction
=
1 - (tokens_delivered + decision_tokens) / raw_tokens

Secondary metric:

Task Success Preservation
=
successful tasks with tpt-weave
/
successful baseline tasks

Never optimise token reduction independently of task success.

24. Target Performance

Initial targets:

Metric

Target

Repository indexing

< 10 s for medium Rust repo

Incremental indexing

< 1 s typical change

Metadata lookup

< 10 ms local

Symbol lookup

< 20 ms local

Cached context

< 50 ms

JEv decision

< 1 s end-to-end

Initial context reduction

> 50%

Mature context reduction

> 75%

Correctness regression

0 tolerated

The >75% target is an engineering objective, not a guaranteed result.

25. Crate Architecture

Start as one repository with a workspace.

Suggested crates:

crates/
├── tpt-weave-core
├── tpt-weave-index
├── tpt-weave-rust
├── tpt-weave-graph
├── tpt-weave-context
├── tpt-weave-decisions
├── tpt-weave-openrouter
├── tpt-weave-cache
├── tpt-weave-tools
├── tpt-weave-mcp
├── tpt-weave-cli
└── tpt-weave-eval

Do not split these into separate repositories prematurely.

26. Core Data Types

Illustrative design:

pub struct RepositoryIndex {
    pub repository: RepositoryId,
    pub revision: Revision,
    pub files: Vec<FileRecord>,
    pub symbols: SymbolIndex,
    pub dependencies: DependencyGraph,
}

pub struct ContextCandidate {
    pub id: ContextId,
    pub source: ContextSource,
    pub level: ContextLevel,
    pub token_estimate: u32,
    pub relationships: Vec<ContextId>,
}

pub enum ContextLevel {
    Metadata,
    Symbols,
    Signatures,
    Skeleton,
    Implementation,
    Full,
}

pub struct DecisionRequest {
    pub question: String,
    pub choices: Vec<String>,
    pub context: String,
}

pub struct Decision {
    pub choice: String,
    pub confidence: f32,
}

Exact schemas should be finalised during implementation.

27. Security

Do not send secrets to JEv or external LLMs.

Before remote decision calls:

redact environment variables;

redact credentials;

redact API keys;

redact private certificates;

apply configurable path exclusions;

support fully local operation.

Repositories may contain private IP.

The context policy must therefore support:

[privacy]
remote_decisions = true
exclude = [
    ".env",
    "secrets/",
    "credentials/",
    "*.pem"
]

28. Adoption Model

A repository should become tpt-weave compatible through:

tpt-weave init
tpt-weave index
tpt-weave doctor

Then the coding environment connects to the MCP/CLI interface.

No application-code rewrite should be necessary.

For high-value repositories, add optional native metadata hooks.

29. Example: tpt-cv

Before:

Agent
  -> inspect repository
  -> list files
  -> open Cargo.toml
  -> search image
  -> open image.rs
  -> open resample.rs
  -> inspect math dependency
  -> inspect tests
  -> inspect more files

With tpt-weave:

Agent
  -> tpt_repo_overview
  -> tpt_find_symbol("resample")
  -> tpt_get_skeleton(...)
  -> tpt_dependencies(...)
  -> tpt_expand(...)

The repository index supplies the graph and the agent receives only the relevant context.

30. Example: tpt-teleop-agri

A task involving weed detection and vehicle control may require:

tpt-teleop-agri
    |
    +--> tpt-cv
    |
    +--> tpt-math
    |
    +--> navigation
    |
    +--> control

tpt-weave should traverse only the dependency branches relevant to the task.

For a CV bug, control code should remain out of context unless requested.

31. Relationship to JEv

JEv is a component.

tpt-weave is the infrastructure.

JEv
= makes fast structured decisions

tpt-weave
= constructs the state on which those decisions operate

This distinction prevents vendor/model lock-in.

32. Relationship to tpt-code-command-center

tpt-code-command-center should become the user-facing coding environment/orchestrator.

tpt-weave should provide its context intelligence.

Recommended architecture:

tpt-code-command-center
        |
        v
    tpt-weave
        |
   +----+----+
   |         |
  JEv      indexes
   |         |
   +----+----+
        |
       GLM

Command Center should not duplicate repository indexing logic.

33. Relationship to tpt-raglite

tpt-raglite can provide generic local retrieval where useful.

tpt-weave should remain source-aware.

Use:

deterministic symbol/index retrieval first;

tpt-raglite for semantic retrieval where exact structural lookup is insufficient.

Do not embed a vector database requirement into the core.

34. Long-Term Direction

Once mature, tpt-weave can become the standard AI interface for the entire TPT ecosystem.

The ideal result is:

Every TPT repository
        |
        v
standard machine-readable representation
        |
        v
standard context interface
        |
        v
JEv / other decision model
        |
        v
any capable coding LLM

This turns the TPT repository ecosystem itself into an AI-efficient development platform.

35. Definition of Done

tpt-weave is ready for broad TPT adoption when:

Rust repositories index automatically;

dependency graphs are accurate;

symbols are searchable;

hierarchical context works;

MCP works;

JEv integration works;

OpenRouter integration works;

tool output reduction works;

cache invalidation is reliable;

token accounting is available;

benchmarks demonstrate material net reduction;

real TPT coding tasks show no unacceptable correctness regression;

integration requires little or no source-code modification.