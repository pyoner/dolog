# AGENTS.md

## Project

`dolog` is a Rust monorepo focused on capturing changes from user SQLite databases.
The system uses SQLite triggers to detect inserts, updates, and deletes, then exports those captured changes to log files or third-party APIs.

Current product shape:
- Monorepo
- CLI-first
- Storage target: SQLite
- Build tool: Cargo

## Current Milestone

The first milestone is a simple CLI that can create, update, and delete triggers for a SQLite database.

Until more structure exists, optimize for:
- Clear CLI UX
- Safe SQLite trigger management
- Minimal, well-factored Rust code
- Easy future expansion toward log export pipelines

## Agent Expectations

Agents may:
- Add dependencies when justified
- Update Cargo configuration
- Change CI configuration
- Create or modify database-related code

Agents do not have protected paths or restricted directories in this repo at the moment.

## Engineering Rules

Use best current Rust practices:
- Prefer stable Rust
- Keep modules small and focused
- Favor explicit types and clear error handling
- Use `Result`-based error propagation instead of panics in normal flows
- Prefer `clap` for CLI ergonomics if a CLI framework is needed
- Prefer `thiserror` or similarly lightweight error types for domain errors
- Keep SQLite operations explicit and reviewable
- Write code that is straightforward to test

Architecture guidance:
- Keep CLI parsing separate from trigger-management logic
- Isolate SQLite-specific logic behind a small internal API
- Design for future support of additional sinks such as files and third-party APIs
- Avoid premature abstraction until the trigger lifecycle is working end to end

## Working Style

Because repository conventions are still forming:
- Keep changes brief and easy to review
- Prefer practical defaults over speculative framework setup
- Document new commands and structure as they are introduced
- When introducing tooling, choose the smallest reasonable addition

## Commands

No canonical dev, test, or run commands are defined yet.
If you add them, update this file so future agents can rely on them.
