# Security policy

## Reporting a vulnerability

If you find a vulnerability or serious issue related to:

- code execution,
- unsafe memory handling,
- unintended data exposure,
- misuse of Win32 processes or handles,

do **not** open a public issue first.

Contact the maintainer through the appropriate private channel on GitHub, or open a security advisory on the repository if available.

## What to include

- description of the impact
- affected version or commit
- reproduction steps
- workaround if available
- minimal proof of concept

## Scope

This project is a local Windows desktop application. The most valuable reports relate to memory safety, local persistence, process integrity, or unsafe interactions with Win32 APIs.
