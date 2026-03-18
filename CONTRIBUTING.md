# Contributing to AI Ranger

Thank you for your interest in contributing to AI Ranger. This guide covers everything you need to get started, from setting up your development environment to submitting a pull request.

AI Ranger is a passive network observability tool that detects AI provider usage via TLS SNI hostname extraction. It is licensed under Apache-2.0.

## Getting Started

### Prerequisites

Ensure you have the following installed:

- **Docker** 24+ (with Docker Compose)
- **Node.js** 22+
- **Rust** 1.75+
- **Go** 1.22+
- **Python** 3.12+
- **protoc** 3+ (Protocol Buffers compiler)

Install scripts for each dependency are available in [`scripts/install-deps/`](scripts/install-deps/).

### Starting the Development Stack

AI Ranger runs as an 8-service stack (nginx, Postgres, ClickHouse, RabbitMQ, gateway, ingest-worker, api-server, dashboard). Start everything with:

```bash
make dev
```

Once the stack is running, verify that your environment is working correctly:

```bash
make test
```

## Running Tests

Each component has its own test commands. You can run them individually or all at once.

### Agent (Rust)

```bash
cargo test
```

The agent has 49 unit tests covering packet capture, SNI extraction, DNS parsing, provider classification, and process resolution. No elevated privileges are needed for unit tests.

### Gateway (Python)

```bash
cd gateway
ruff check .
mypy .
```

### Workers (Go)

```bash
cd workers
go test ./...
```

### Dashboard (TypeScript)

```bash
cd dashboard
npm run lint
```

### All Components

```bash
make test
```

### Integration Tests

The full integration test suite exercises the entire pipeline from agent through ClickHouse. It requires sudo because the agent needs raw socket access for packet capture.

```bash
make test-integration
```

## Adding a Provider

Provider additions are the easiest and most welcome contribution. If you know of an AI service that AI Ranger does not yet detect, adding it is straightforward.

For the full format specification and detailed instructions, see [`providers/CONTRIBUTING.md`](providers/CONTRIBUTING.md).

Here is a brief summary:

1. Open `providers/providers.toml`.
2. Add a new `[[providers]]` block with the required fields:

```toml
[[providers]]
name = "provider-slug"
display_name = "Provider Display Name"
hostnames = [
    "api.provider.com",
    "inference.provider.com",
]
```

- `name` -- A lowercase, hyphenated slug used as the internal identifier.
- `display_name` -- The human-readable name shown in the dashboard and event output.
- `hostnames` -- A list of hostnames that the provider uses. The classifier matches both exact hostnames and subdomains.

3. Run unit tests to verify the provider is recognized:

```bash
cargo test
```

4. Run integration tests to confirm end-to-end detection:

```bash
make test-integration
```

If you are unsure which hostnames a provider uses, open a GitHub Issue describing the provider and we can help research it.

## Submitting a Pull Request

### Branching

Branch from `main` and use a descriptive branch name:

- `feat/add-provider-x` -- for new features or providers
- `fix/enrollment-race` -- for bug fixes
- `docs/update-architecture` -- for documentation changes
- `refactor/extract-sink-trait` -- for refactoring

### Commit Messages

Write clear, concise commit messages. Use the imperative mood (e.g., "add provider for Mistral" not "added provider for Mistral"). If a commit addresses a GitHub Issue, reference it in the message body.

### CI Requirements

All of the following checks must pass before a PR can be merged:

| Component   | Checks                                                        |
|-------------|---------------------------------------------------------------|
| Agent       | `cargo build`, `cargo clippy -- -D warnings`, `cargo test` on Linux, macOS, and Windows |
| Gateway     | `ruff check .`, `mypy .`                                      |
| Workers     | `go vet ./...`, `golangci-lint run`, `go test ./...`          |
| Dashboard   | `npm run build`, `npm run lint`                               |
| Integration | Full pipeline test suite                                      |

### PR Description

Include in your pull request:

- **What changed** -- A summary of the modifications and the motivation behind them.
- **How to test** -- Steps a reviewer can follow to verify the change works correctly.
- **Related issues** -- Link any relevant GitHub Issues.

## Code Style

### Rust (Agent)

- Format with `rustfmt`. Run `cargo fmt --check` before committing.
- Lint with `cargo clippy -- -D warnings`. All warnings are treated as errors.
- Never use `.unwrap()` in production code paths. Use proper error handling with `Result` and the `?` operator.
- Use `pub(crate)` by default. Only use `pub` when the item is part of a deliberate public interface.
- No magic numbers or magic strings. Define named constants with doc comments.
- Keep functions under 50 lines unless sequential steps require otherwise.

### Python (Gateway)

- Lint with `ruff`. All rules must pass with zero violations.
- Use type annotations on all function signatures and variables where the type is not obvious.
- Use `pydantic-settings` for runtime configuration. No hardcoded hostnames, ports, or credentials.
- SQLAlchemy 2.0 async for database access. No raw SQL outside of Alembic migration files.

### Go (Workers)

- Format with `gofmt`. Run `gofmt -l .` to check for unformatted files.
- Lint with `go vet` and `golangci-lint`.
- All exported functions and types must have doc comments.
- No `os.Getenv` outside the config package. All environment variable access is centralized in the config struct loaded at startup.
- GORM for Postgres access. ClickHouse queries use the `clickhouse-go` driver with named query constants.

### TypeScript (Dashboard)

- Lint with ESLint. All rules must pass.
- Use functional React components. No class components.
- Use TanStack Query for all data fetching.
- No inline styles -- use the project's styling approach consistently.

## Where to Get Help

- **Bugs** -- Open a [GitHub Issue](../../issues) with steps to reproduce, expected behavior, and actual behavior.
- **Questions** -- Start a thread in [GitHub Discussions](../../discussions) for design questions, usage help, or ideas for new features.

We appreciate every contribution, whether it is a one-line provider addition or a multi-component feature. Thank you for helping make AI Ranger better.
