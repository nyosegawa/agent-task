# AGENTS.md
## Absolute Rules
1. Instruction budget rule: keep this AGENTS.md in 20-30 lines.
2. No backward compatibility: never preserve legacy behavior/interfaces.
3. Commit and push regularly in small, reviewable increments.
4. Refactor periodically to reduce complexity and technical debt.
5. Always use the latest versions for libraries/dependencies, and research their usage thoroughly before introducing them.
## Development Policy
1. This repository is TDD-first by default.
2. Always execute Red -> Green -> Refactor.
3. Start with a failing test before production code.
4. If tests are hard to write, create seams (split function, trait, adapter) and test through them.
## Required Flow Per Change
1. Define expected behavior as a test.
2. Run tests and confirm failure (Red).
3. Implement the minimal fix (Green).
4. Refactor safely with tests green.
5. Re-run full relevant scope before finishing.
## Rust Verification Loop
1. `cargo test`
2. `cargo fmt --check`
3. `cargo clippy -- -D warnings`
## Task CLI Specifics
- Storage: `~/.local/share/tasks/tasks.log` (append-only JSONL, never edit/delete entries)
- Project ID: `git remote get-url origin` → `owner/repo` format; fallback to cwd path
- stdout output is the contract: `TASK_ADD_{id}`, `TASK_{STATUS}_{id}` (e.g. `TASK_DOING_{id}`, `TASK_INREVIEW_{id}`)
