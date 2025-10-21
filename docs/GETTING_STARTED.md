# Getting Started with Yatagarasu

Welcome to Yatagarasu! This guide will help you understand the project structure and start development following Kent Beck's TDD methodology.

## What You've Got

Your Yatagarasu S3 proxy project now includes:

### 📄 Core Documentation

1. **[README.md](README.md)** - Project overview, features, and quick start
   - What Yatagarasu is and what it does
   - Installation and basic usage
   - Architecture and use cases

2. **[CLAUDE.md](CLAUDE.md)** - Development methodology guide ⭐ **READ THIS FIRST**
   - Kent Beck's TDD principles (Red → Green → Refactor)
   - "Tidy First" approach (structural vs behavioral changes)
   - Commit discipline and code quality standards
   - Detailed workflow examples

3. **[spec.md](spec.md)** - Complete product specification
   - Functional requirements (multi-bucket routing, JWT auth, S3 proxying)
   - Non-functional requirements (performance, security, reliability)
   - Technical specifications (architecture, tech stack, APIs)
   - Error handling and testing strategy
   - Data models and configuration schemas

4. **[plan.md](plan.md)** - TDD implementation plan ⭐ **YOUR ROADMAP**
   - 200+ tests organized in phases
   - Phase 1: Foundation (project setup)
   - Phase 2: Configuration management
   - Phase 3: Path routing
   - Phase 4: JWT authentication
   - Phase 5: S3 integration
   - Phase 6: Pingora proxy integration
   - Phase 7-11: Integration tests, performance, production readiness
   - Test execution commands

5. **[config.yaml](config.yaml)** - Example configuration
   - Complete configuration examples for all features
   - Public and private bucket configurations
   - JWT authentication examples
   - Environment variable usage

## Quick Start

### Step 1: Understand the Methodology

Read `CLAUDE.md` to understand the TDD approach:

```bash
cat CLAUDE.md
```

Key principles:
- 🔴 **Red**: Write a failing test first
- 🟢 **Green**: Make it pass with minimum code
- 🔵 **Refactor**: Clean up while keeping tests green
- 💾 **Commit**: Separate [STRUCTURAL] and [BEHAVIORAL] commits

### Step 2: Review the Specification

Read `spec.md` to understand what you're building:

```bash
cat spec.md
```

Yatagarasu is an S3 proxy that provides:
- Multi-bucket routing with path prefixes
- Flexible JWT authentication
- S3 request proxying with AWS Signature v4
- Hot configuration reload
- Prometheus metrics

### Step 3: Check the Implementation Plan

Open `plan.md` to see the test roadmap:

```bash
cat plan.md
```

The plan has 11 phases with 200+ tests. You'll implement them one at a time.

### Step 4: Start Development

#### Option A: Work with Claude (AI Assistant)

Simply say:
```
go
```

Claude will:
1. Read CLAUDE.md and plan.md
2. Find the next unmarked test `[ ]`
3. Implement the test (Red)
4. Implement the minimum code (Green)
5. Refactor if needed
6. Mark test complete `[x]` and commit
7. Ask for next "go"

#### Option B: Work Manually

1. Find the next `[ ]` test in plan.md
2. Write the test (should fail)
3. Run `cargo test` - confirm it fails
4. Write minimum code to pass
5. Run `cargo test` - confirm it passes
6. Refactor if needed
7. Mark test `[x]`
8. Commit with proper prefix:
   ```bash
   git commit -m "[BEHAVIORAL] Add JWT validation from header"
   ```

## Project Structure

```
yatagarasu/
├── CLAUDE.md           # Development methodology ⭐ READ FIRST
├── spec.md             # Product specification
├── plan.md             # Test roadmap ⭐ YOUR GUIDE
├── README.md           # Project overview
├── config.yaml         # Example configuration
├── Cargo.toml          # Rust dependencies (create this)
└── src/                # Source code (create via TDD)
    ├── main.rs
    ├── lib.rs
    ├── config/         # Configuration loading
    ├── router/         # Path routing
    ├── auth/           # JWT authentication
    ├── s3/             # S3 client
    └── error.rs        # Error types
```

## The First Tests

Your first tests (Phase 1) are:

```
Phase 1: Foundation and Project Setup
├── [ ] Test: Cargo project compiles without errors
├── [ ] Test: Basic dependency imports work (Pingora, Tokio)
├── [ ] Test: Can run `cargo test` successfully
├── [ ] Test: Can run `cargo clippy` without warnings
└── [ ] Test: Can run `cargo fmt --check` successfully
```

These are simple but important - they establish your development environment.

## Example TDD Cycle

Let's walk through the first test:

### 1. Red Phase (Failing Test)

```rust
// tests/project_setup.rs
#[test]
fn test_project_compiles() {
    // This test just needs to compile
    assert!(true);
}
```

Run: `cargo test` → Should fail (no project yet!)

### 2. Green Phase (Make It Pass)

Create minimal `Cargo.toml`:
```toml
[package]
name = "yatagarasu"
version = "1.0.0"
edition = "2021"

[dependencies]
```

Create minimal `src/main.rs`:
```rust
fn main() {
    println!("Yatagarasu starting...");
}
```

Run: `cargo test` → Should pass!

### 3. Refactor Phase

No refactoring needed for this simple test.

### 4. Commit

```bash
git add .
git commit -m "[BEHAVIORAL] Initialize Cargo project"
```

### 5. Mark Complete

In plan.md, change:
```
- [ ] Test: Cargo project compiles without errors
```
to:
```
- [x] Test: Cargo project compiles without errors
```

### 6. Next Test

Move to the next unmarked test and repeat!

## Key Reminders

### ✅ DO

- Write one test at a time
- Make tests fail first (Red)
- Write minimum code to pass (Green)
- Refactor only when tests are green
- Commit frequently with clear messages
- Mark tests complete in plan.md
- Run all tests before committing
- Keep structural and behavioral commits separate

### ❌ DON'T

- Skip tests or write code without tests
- Write more code than needed to pass the test
- Commit when tests are failing
- Commit when there are compiler/linter warnings
- Mix structural and behavioral changes in one commit
- Guess at requirements - refer to spec.md

## Commit Message Format

Every commit must have a prefix:

```bash
# Behavioral changes (add/modify functionality)
[BEHAVIORAL] Add JWT token extraction from header
[BEHAVIORAL] Implement S3 signature generation
[BEHAVIORAL] Fix routing bug for nested paths

# Structural changes (refactor/reorganize)
[STRUCTURAL] Extract validation logic to separate function
[STRUCTURAL] Rename Config to ProxyConfig for clarity
[STRUCTURAL] Move auth module to separate file
```

## Tools and Commands

```bash
# Testing
cargo test                    # Run all tests
cargo test --lib             # Unit tests only
cargo test --test '*'        # Integration tests only
cargo test jwt_validation    # Specific test

# Code Quality
cargo clippy                 # Linter
cargo fmt                    # Format code
cargo tarpaulin --out Html   # Coverage report

# Build
cargo build                  # Debug build
cargo build --release        # Release build

# Run
cargo run -- --config config.yaml
```

## Getting Help

### Documentation

- **CLAUDE.md**: How to develop (methodology)
- **spec.md**: What to build (requirements)
- **plan.md**: What to do next (tests)
- **README.md**: How to use (operations)

### Ask Claude

If working with Claude AI, you can ask:
- "What's the next test?"
- "Explain this requirement"
- "How should I structure this code?"
- "Help me refactor this"
- "Review my implementation"

Just say **"go"** to start the next test!

## Success Criteria

You'll know you're on track when:
- ✅ All tests are passing
- ✅ No compiler warnings
- ✅ No clippy warnings
- ✅ Tests marked complete in plan.md
- ✅ Code is well-structured and readable
- ✅ Commits follow the format
- ✅ Making steady progress through phases

## What's Next?

1. **Now**: Read CLAUDE.md thoroughly
2. **Next**: Review spec.md to understand requirements
3. **Then**: Open plan.md and find the first test
4. **Finally**: Start the TDD cycle!

---

**Ready to begin?**

If you're working with Claude AI, just say:
```
go
```

If you're working manually, open plan.md and implement the first test!

Good luck building Yatagarasu! 🚀

Remember: Quality is built in from the first test, not added later.
