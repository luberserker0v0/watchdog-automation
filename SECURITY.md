# Security policy

## Supported versions

WatchDog has not published a stable release. Until the first stable release,
security fixes are applied only to the latest revision of the default branch.

After stable releases begin, this table will identify supported release lines:

| Version | Supported |
| --- | --- |
| Unreleased default branch | Yes |
| Older development snapshots | No |

## Report a vulnerability

Use GitHub's private vulnerability reporting for this repository:

[Report a vulnerability privately](https://github.com/luberserker0v0/watchdog-automation/security/advisories/new)

Include:

- the affected revision or version;
- operating system and architecture;
- reproduction steps or a minimal pipeline;
- expected and observed behavior;
- security impact;
- logs with credentials and personal information removed.

Do not open a public issue containing API keys, captured screenshots, local
paths containing personal information, or instructions that make exploitation
easier. If private reporting is unavailable, open a public issue requesting a
private contact channel without including vulnerability details.

## Sensitive data handled by WatchDog

WatchDog can process screenshots, window titles, keyboard input, pipeline
parameters, VLM credentials, and generated debug evidence. Reports should use
the sample Target application whenever possible.

The Patrol API key is session-only and must not appear in logs, persisted
preferences, status events, or Chat Room events. Recorded text is masked by
default. Contributors must preserve these properties when changing telemetry,
serialization, or error handling.

## Disclosure process

The maintainers will acknowledge a complete report, reproduce the problem,
prepare a fix and regression test, and coordinate disclosure after affected
users have an upgrade path. Exact response times are not promised before a
maintained stable release exists.
