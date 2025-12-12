# exe-runners

Task execution and management utilities with optional Reth integration.

## Overview

This crate provides task runners and shutdown management functionality, with optional integration for Reth's task system. It offers a flexible execution framework for managing asynchronous tasks and graceful shutdown handling.

## Features

- `reth-tasks` - Use Reth's task system instead of built-in implementation
- `rayon` - Parallel execution support via Rayon

## Supported Functionality

- Task execution and management
- Graceful shutdown handling
- Runner abstractions for async operations
- Compatible with both standalone use and Reth integration