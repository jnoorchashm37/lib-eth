# lib-eth: A Set of Ethereum Tooling Libraries

A collection of Ethereum libraries for building applications with Reth and Alloy.

## Overview

This monorepo contains utilities and abstractions for interacting with Ethereum networks, Reth nodes, and Uniswap protocols. It provides a comprehensive toolkit for developers building on Ethereum and Layer 2 networks.

## Crates

### [lib-reth](./crates/lib-reth/)
Unified interface for connecting to and interacting with Reth nodes via HTTP, IPC, WebSocket, or direct database access.

### [eth-network-exts](./crates/eth-network-exts/)
Network extension types for Ethereum and L2 chains, supporting Ethereum mainnet, Base, Sepolia, and Unichain.

### [uniswap-storage](./crates/uniswap-storage/)
Storage utilities for fetching and managing data from Uniswap V3, V4, and Angstrom protocols.

### [exe-runners](./crates/exe-runners/)
Task execution and management utilities with optional Reth integration for async operations and graceful shutdown.
