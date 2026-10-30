# Security Policy

## Scope

This document covers the security policy for **soroban-lint** — the static analyzer tool itself.

There are three distinct situations addressed below. Please read carefully to determine which applies to you.

---

## Vulnerabilities IN soroban-lint

If you have found a security vulnerability **in the soroban-lint tool itself** — for example, a crafted input file that causes a crash, arbitrary code execution, path traversal during file discovery, or any other unsafe behavior — please **do not open a public GitHub issue**.

Report it privately instead:

> **Report security issues to: [SECURITY_EMAIL_PLACEHOLDER — fill in before first release]**

Please include:

- A description of the vulnerability and its potential impact.
- A minimal reproducing example (file, command, or configuration) that demonstrates the issue.
- The version of soroban-lint you are using (`soroban-lint --version`).
- Your operating system and Rust toolchain version.

We will acknowledge receipt within **5 business days** and work with you to understand and address the issue before any public disclosure. We aim to release a fix within 30 days for confirmed critical vulnerabilities, and will coordinate a disclosure timeline with you.

We ask that you give us a reasonable amount of time to address the issue before any public disclosure. We will credit reporters in the release notes unless you prefer to remain anonymous.

---

## Vulnerabilities FOUND using soroban-lint in someone else's contract

If you used soroban-lint to identify a vulnerability in a **third-party Soroban contract**, please practice responsible disclosure:

- **Do not open a public issue** on this repository disclosing details of an unpatched vulnerability in a deployed contract.
- **Contact the maintainers** of that contract project privately first, giving them time to assess and patch the issue before public disclosure.
- Follow the disclosure policy of the project you are reporting to, if they have one.

If you are unsure how to proceed, feel free to reach out to us at the security address above for general guidance.

---

## Missed vulnerabilities (false negatives)

If soroban-lint failed to detect a vulnerability that represents a **general, repeatable pattern** (not a specific unpatched deployed contract), you are welcome to open a public issue or pull request:

- Describe the pattern and provide a minimal, synthesized example contract.
- Explain why soroban-lint should detect it (which rule, or a new rule).
- Do **not** include details that could be used to exploit a specific live contract.

These contributions help improve the tool for everyone and are greatly appreciated.

---

## Supported Versions

| Version | Supported |
|---|---|
| `0.1.x` (current) | ✅ |
| Earlier | ❌ |

---

## Response Time Pledge

| Action | Target |
|---|---|
| Acknowledge receipt of private report | 5 business days |
| Provide an initial assessment | 10 business days |
| Release a fix (critical vulnerabilities) | 30 days from confirmation |
