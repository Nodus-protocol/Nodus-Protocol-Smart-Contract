# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest (`main`) | ✅ |
| Older branches | ❌ |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Report them privately by opening a
[GitHub Security Advisory](../../security/advisories/new) in this repository.

Include as much of the following information as possible:

- Type of vulnerability, using the smart contract categories below where possible
- Full paths of source files related to the vulnerability
- Location of the affected source code (tag, branch, commit, or direct URL)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the vulnerability and how an attacker might exploit it

## Smart Contract Vulnerability Categories

When reporting issues with the Soroban AMM contract, please include the most
relevant category:

| Category | Description | Example |
|----------|-------------|---------|
| Reentrancy | Cross-contract callbacks re-enter pool logic before state is committed | Token transfer callback attempts to call `swap` again |
| AMM Math | Overflow, underflow, rounding, or precision loss in reserve calculations | Constant-product invariant check overflows at large reserves |
| Price Manipulation | Flash-loan or multi-step reserve manipulation that extracts value | Temporary reserve imbalance lets an attacker receive excess output |
| LP Token Accounting | Incorrect mint, burn, or share calculation for liquidity providers | Tiny deposits mint zero shares or withdrawals burn the wrong amount |
| Allowance Race | Approval or transfer-from ordering creates a front-running window | Spender uses an old allowance before a new one is applied |
| Deadline Bypass | Expired transactions are accepted or checked against the wrong clock | Swap succeeds after the user-provided deadline |
| Storage Expiry | Soroban storage TTL behavior can remove required contract state | Pool or LP balance data expires before a valid operation |
| Initialization | Contract can be re-initialized or initialized with invalid state | Admin, token, or reserve addresses are missing or duplicated |

This contract is not yet audited. We do not currently operate a formal bug
bounty program, but critical vulnerability reporters will be credited in the
project's security advisories and acknowledged in release notes unless they
prefer to remain anonymous.

We will acknowledge your report within **48 hours** and aim to release a patch
within **14 days** for critical issues. You will be credited in the release
notes unless you prefer to remain anonymous.

## Disclosure Policy

Once a fix is ready and deployed, we will:
1. Publish a GitHub Security Advisory with full details
2. Credit the reporter (unless they opt out)
3. Tag a new release with the fix

We ask that you give us reasonable time to patch before any public disclosure.
