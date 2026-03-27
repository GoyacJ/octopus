# AGENTS.md

## Purpose

These instructions apply to work under `schemas/interop/`.

## Local Rules

- Keep only Hub/Client transport-parity contracts in this directory.
- Use this group for shared connection-status or stream-event payloads that do not fit cleanly into runtime, governance, or observe object ownership.
- Do not move runtime records, approval objects, or observation records here just because they cross HTTP or Tauri transports.

## Current Files

- `hub-connection-status.schema.json`
- `hub-event.schema.json`
