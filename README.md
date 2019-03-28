# prometheus-query

Asynchronous Prometheus library for the Prometheus V1 HTTP API.
This library is **nightly-only**, and built using **experimental** `futures-0.3` and `async`/`await`.

## Status

This code is a **WIP**, and does not have rustdoc, examples,
or unit/integration tests. It has two components:

- A library that defines an async interface for making queries or commands
- A cli that uses this library

## Queries

The following query types are supported by the library:

- [x] Instant
- [x] Range
- [x] Series 
- [x] Label names
- [x] Label values
- [x] Targets
- [x] Alertmanagers
- [x] Status
- [ ] Config
- [x] Flags


## Commands

The following commands are supported by the library:

- [x] Delete series
- [ ] Snapshot
- [ ] Clean tombstones

## CLI

The CLI exposes the following queries/commands:

- [x] Instant
- [ ] Range
- [ ] Series 
- [ ] Label names
- [ ] Label values
- [ ] Targets
- [ ] Alertmanagers
- [ ] Status
- [ ] Config
- [ ] Flags
- [x] Delete series
- [ ] Snapshot
- [ ] Clean tombstones
