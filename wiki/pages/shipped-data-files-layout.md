---
title: Shipped Data Files Layout
type: reference
sources: [S006]
updated: 2026-10-06
---

Verified by downloading the published `data-files.zip` (S006):

| Path in the archive | Contents |
|---|---|
| `static/layouts/english/*.dof` | 146 layout files |
| `static/language_data/english.json` | language data |
| corpus configs | 58 `.toml` files |

All three paths that `default_config_toml` generates resolve against the real
release, so the default config is not broken (S006).

## A partial listing manufactures a phantom bug

`english` looked absent from a partial listing of the shipped layout directories,
which reads exactly like a defect — the generated layout path points at a
directory that does not exist. The archive ships it, with 146 `.dof` files
(S006).

Recorded so it is not re-investigated: check the real archive before treating a
generated path as broken, and never conclude absence from a partial listing
(S006).

Related: [Generated Config Round-Trip](./generated-config-round-trip.md) ·
[Data Archive Pipeline](./data-archive-pipeline.md)
