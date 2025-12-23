---
trigger: always_on
---

# Rust Project Rules

## Toolchain & Formatting
- Use Rust 2021 edition and stable toolchain.
- All code must pass `cargo fmt` and `cargo clippy --all-targets --all-features`.

## Code Style
- Follow the Rust API Guidelines.
- Max line length: 100 characters.
- Prefer iterators and combinators over manual loops where it improves clarity.
- Avoid unnecessary `clone`; think in terms of ownership and borrowing.

## Types & APIs
- Prefer strong types over `String` / `&str` for domain concepts (use newtypes).
- Provide clear, minimal public APIs; avoid exposing internal types unnecessarily.
- Implement `From` / `TryFrom` for fallible conversions where appropriate.

## Error Handling
- All fallible operations return `Result<T, E>`.
- Use `thiserror` (libraries) or `anyhow` (apps) for error definitions and context.
- Use the `?` operator for error propagation and keep error paths explicit.

## Async & Concurrency
- Use Tokio as the async runtime (if async is needed).
- Do not block the async runtime; use `spawn_blocking` for CPU-bound work.
- Prefer message passing (channels) over shared mutable state; if using locks, justify in comments.

## Testing & Performance
- New features must include unit tests; public APIs should have integration tests in `tests/`.
- For performance-critical code paths, add benchmarks and document assumptions.
- Use property-based tests for complex invariants when useful.

## Project Conventions
- Keep modules small and cohesive; avoid "god modules".
- Document public items with Rustdoc comments.
- When modifying existing code, keep the existing style consistent.
