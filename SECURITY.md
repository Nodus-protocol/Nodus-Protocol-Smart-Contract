# Security Policy

## Supported Versions

| Version         | Supported |
| --------------- | --------- |
| Latest (`main`) | ✅        |
| Older branches  | ❌        |

## Responsible Disclosure Process

### Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

We take all security vulnerabilities seriously. If you discover a security issue in the Nodus Protocol Smart Contract, please report it privately through one of the following methods:

1. **Preferred:** Open a [GitHub Security Advisory](../../security/advisories/new) in this repository
2. **Alternative:** Email the security team at security@nodusprotocol.com (if available)

### What to Include in Your Report

To help us understand and address the vulnerability quickly, please include as much of the following information as possible:

- **Type of vulnerability** (e.g., reentrancy, integer overflow, access control bypass)
- **Full paths of source files** related to the vulnerability
- **Location of the affected source code** (tag, branch, commit, or direct URL)
- **Step-by-step instructions** to reproduce the issue
- **Proof-of-concept or exploit code** (if possible)
- **Impact assessment**: How an attacker might exploit this vulnerability
- **Suggested fix** (if you have one)

### Our Commitment

- **Acknowledgment:** We will acknowledge your report within **48 hours**
- **Updates:** We will provide regular updates on our progress
- **Fix Timeline:** We aim to release a patch within **14 days** for critical issues
- **Credit:** You will be credited in the release notes unless you prefer to remain anonymous

## Vulnerability Severity Classification

We use the following severity levels:

- **Critical:** Immediate threat to user funds or contract integrity (e.g., fund drainage, unauthorized minting)
- **High:** Significant impact on contract functionality or user assets
- **Medium:** Limited impact or requires specific conditions to exploit
- **Low:** Minimal impact or theoretical issues

## Disclosure Policy

Once a fix is ready and deployed, we will:

1. Publish a GitHub Security Advisory with full details
2. Credit the reporter (unless they opt out)
3. Tag a new release with the fix
4. Notify affected users if applicable

We ask that you give us reasonable time to patch the vulnerability before any public disclosure. We follow a **90-day disclosure timeline** from the initial report, or until a fix is deployed, whichever comes first.

## Security Best Practices for Contributors

- All arithmetic operations must use `checked_*` methods
- Follow the Checks-Effects-Interactions pattern
- Always use the reentrancy guard for state-changing functions
- Include comprehensive tests for security-critical code
- Document security assumptions in code comments

## Additional Security Resources

For detailed information about our security architecture and known attack vectors, see [docs/SECURITY.md](docs/SECURITY.md).
