# lib-reth

Unified interface for connecting to and interacting with Reth nodes.

## Overview

This crate provides a comprehensive library for connecting to Ethereum endpoints using Reth, supporting both Ethereum mainnet and Optimism L2 networks.

## Connection Types

- HTTP (default)
- IPC (feature = `ipc`)
- WebSocket (feature = `ws`)
- Direct database access via libmdbx (feature = `reth-libmdbx`)

## Features

- `full` - All connection types and integrations
- `revm` - REVM execution support
- `op-reth-libmdbx` - Optimism node support
- `rayon` - Parallel execution support

## Supported Functionality

- RPC client implementations
- Streaming support for blocks, transactions, and logs
- Direct database access for local nodes
- Integration with Uniswap storage utilities