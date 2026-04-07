# Security Policy

## Reporting a vulnerability

If you find a vulnerability or serious issue related to:

- code execution,
- unsafe memory handling,
- unintended data exposure,
- misuse of Win32 processes or handles,

please **do not open a public issue first**.

Instead, contact the maintainer through the appropriate private channel on GitHub, or open a security advisory on the repository if available.

## What to include

- description of the impact,
- affected version/commit,
- reproduction steps,
- workaround if available,
- minimal proof of concept.

## Scope

This project is a local Windows desktop application. The most valuable reports are those related to memory safety, local persistence, process integrity, or unsafe interactions with Win32 APIs.
