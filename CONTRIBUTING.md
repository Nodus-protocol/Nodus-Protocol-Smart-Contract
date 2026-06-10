# Contributing to Nodus Protocol

Thank you for your interest in contributing to Nodus Protocol. This document covers everything you need to get started: workflow, standards, and how to get your PR merged.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
- [Development Workflow](#development-workflow)
- [Commit Message Format](#commit-message-format)
- [Pull Request Guidelines](#pull-request-guidelines)
- [Issue Guidelines](#issue-guidelines)
- [Code Style](#code-style)
- [Testing Requirements](#testing-requirements)
- [Security Vulnerabilities](#security-vulnerabilities)
- [Getting Help](#getting-help)

---

## Code of Conduct

All contributors are expected to treat each other with respect. Harassment, discrimination, or toxic behaviour of any kind will result in an immediate ban. Be kind, be constructive.

---

## Getting Started

### 1. Fork the Repository

Click the **Fork** button at the top right of this page, then clone your fork locally:

```bash
git clone https://github.com/YOUR_USERNAME/REPO_NAME.git
cd REPO_NAME
```

### 2. Add the Upstream Remote

```bash
git remote add upstream https://github.com/Nodus-protocol/REPO_NAME.git
```

### 3. Follow the Setup Instructions

Each repo has a `README.md` with environment setup steps. Complete those before making any changes.

---

## How to Contribute

### Pick an Issue

Browse the [open issues](../../issues) and look for ones labelled `good first issue` or `help wanted`. If you want to work on something, **comment on the issue first** to claim it — this prevents duplicate effort.

If you have a new idea that isn't an existing issue, open an issue and discuss it with the team before writing code. PRs without a linked issue may be closed.

### Claim an Issue

Leave a comment: `I'd like to work on this` and a maintainer will assign it to you. If no progress is made within **7 days**, the issue will be unassigned so others can pick it up.

---

## Development Workflow

### Branch Naming

Always branch from `main`:

```bash
git checkout main
git pull upstream main
git checkout -b type/short-description
```

Branch name format: `type/short-description`

| Type | Use for |
|---|---|
| `feat/` | New features |
| `fix/` | Bug fixes |
| `docs/` | Documentation changes |
| `refactor/` | Code refactoring |
| `test/` | Adding or improving tests |
| `chore/` | Build scripts, config, tooling |

**Examples:**
- `feat/rate-limiting-middleware`
- `fix/refresh-token-reuse`
- `docs/deployment-guide`

### Keep Your Branch Up to Date

Before opening a PR, rebase onto the latest `main`:

```bash
git fetch upstream
git rebase upstream/main
```

Resolve any conflicts, then push:

```bash
git push origin your-branch-name
```

---

## Commit Message Format

We follow the Conventional Commits specification.

**Format:**

```
type(scope): short description

Optional longer body explaining the WHY, not the WHAT.

Closes #123
```

**Types:** `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`

**Examples:**

```
feat(auth): add TOTP two-factor authentication

Adds setup, verify, and disable endpoints for TOTP 2FA.
Users with 2FA enabled receive a short-lived session token on login
and must authenticate the TOTP code before receiving full JWT access.

Closes #10
```

```
fix(swap): add deadline check to prevent stale execution

Closes #2
```

**Rules:**
- Subject line: 72 characters max, imperative mood ("add", not "added")
- Reference the issue number in the footer: `Closes #N`
- One commit per logical change — squash fixup commits before opening a PR

---

## Pull Request Guidelines

### Before Opening a PR

- [ ] All tests pass locally
- [ ] No linting errors
- [ ] The issue is linked in the PR description (`Closes #N`)
- [ ] You have rebased onto the latest `main` (no merge commits)
- [ ] You have tested the change manually (not just tests)

### PR Title

Use the same format as commit messages: `type(scope): description`

### PR Description Template

When opening a PR, use this as your description:

```
## What does this PR do?
A brief description of the change.

## Why?
Link to the issue and explain the motivation.

## How was it tested?
Describe how you tested this change (unit tests, integration tests, manual testing steps).

## Checklist
- [ ] Tests added / updated
- [ ] Documentation updated (if applicable)
- [ ] No new linting warnings
- [ ] Linked issue: Closes #N
```

### Review Process

- A maintainer will review your PR within **72 hours**
- Address all review comments — mark conversations as resolved once fixed
- PRs with unresolved conflicts will not be merged
- Once approved, a maintainer will merge using **squash and merge**

---

## Issue Guidelines

### Reporting a Bug

Use the bug report template. Include:
1. What you expected to happen
2. What actually happened
3. Exact steps to reproduce
4. Environment details (OS, runtime version, etc.)

### Requesting a Feature

Open a feature request issue and explain:
1. The problem you are trying to solve
2. Your proposed solution
3. Any alternatives you considered

Do not open a PR for a feature without first opening and getting acknowledgement on the issue.

---

## Code Style

Each repo has its own language-specific standards, but these apply everywhere:

- **Clarity over cleverness** — Write code that a new contributor can understand in 5 minutes
- **No commented-out code** — Delete it; git history preserves it
- **No TODO comments** — Open an issue instead
- **Self-documenting names** — Avoid abbreviations and single-letter variables outside of loop indices

Run the linter before every commit:

| Repo | Command |
|---|---|
| Frontend | `npm run lint && npm run typecheck` |
| Backend | `go vet ./... && golangci-lint run` |
| Smart Contract | `cargo clippy -- -D warnings && cargo fmt --check` |
| Core Engine | `cargo clippy -- -D warnings && cargo fmt --check` |
| Mobile | `flutter analyze` |

---

## Testing Requirements

All PRs must include tests appropriate to the change:

| Change Type | Required |
|---|---|
| New feature | Unit tests + integration test (where applicable) |
| Bug fix | Regression test that fails before the fix |
| Refactor | Existing tests must still pass |
| Documentation | No tests needed |

**Coverage:** PRs must not reduce overall test coverage. If you are unsure how to test something, ask in the issue thread before writing the code.

---

## Security Vulnerabilities

**Do not open a public GitHub issue for security vulnerabilities.**

Report them privately by opening a [GitHub Security Advisory](../../security/advisories/new) in this repository. Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact

We will acknowledge within 48 hours and aim to patch within 14 days. You will be credited in the release notes.

---

## Getting Help

- **Issue questions:** Comment directly on the issue
- **General discussion:** Open a [Discussion](../../discussions)
- **Stuck on setup?** Check the `README.md` troubleshooting section first, then open a discussion

We are a welcoming community. No question is too basic — just ask.

---

*Thank you for helping build Nodus Protocol.*
