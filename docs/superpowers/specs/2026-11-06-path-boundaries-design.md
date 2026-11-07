# Path boundaries for names and resource archives

## Intent

Prevent preset and layout names and downloaded archive entries from escaping their intended storage directories (issues #3 and #14), while preserving ordinary names and archive contents.

## Approaches

1. **Selected: reject unsafe inputs at the boundary.** Require names to identify one filename and reject unsafe archive paths before extraction. This is a focused change using existing filesystem operations.
2. Use directory-relative filesystem operations to defend against pre-existing malicious symlinks as well. This has a larger, platform-dependent implementation cost and is outside the reported traversal fixes.

## Design

- Validate every preset and managed-layout name used to form a filesystem path. Reject empty names, absolute paths, separators on either supported platform, and dot or parent-directory components. Preserve existing normalization of valid layout names and return a useful error for rejected names.
- Validate each ZIP entry path, including after the expected prefix is removed, before joining it to the destination. Reject absolute paths and parent-directory traversal rather than silently rewriting them. Preserve legitimate nested entries and the existing prefix behavior.
- Exercise the real path-building and extraction boundaries with temporary directories. Cover normal inputs and malicious paths, and verify neighboring files remain unchanged. Include a regression for the preset overwrite reported in #3.
- Keep error handling explicit. An invalid archive must not be reported as a successful extraction. The implementation should inspect existing extraction and file-writing behavior before choosing the smallest testable boundary.

## Scope and limits

This addresses untrusted path text. It does not defend against malicious symlinks already present in destination directories, or add transactional rollback of files extracted before a later invalid entry. No unrelated issue or new user-facing option is included.
