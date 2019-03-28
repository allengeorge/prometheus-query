# prometheus-query

## Prometheus HTTP Client

Asynchronous Prometheus client for the Prometheus V1 HTTP API.
The client is built using **experimental** futures-0.3 and async/await.

## Status

This code is a **WIP**, and does not have rustdoc, examples,
or unit/integration tests. It has two components:

- A library that defines an async interface for making queries or commands
- A cli that uses this library

## Queries

The following query types are supported:

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

The following commands are supported:

- [x] Delete series
- [ ] Snapshot
- [ ] Clean tombstones
