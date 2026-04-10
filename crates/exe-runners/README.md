# exe-runners

Task execution and CLI runner utilities modeled after `reth-tasks` and
`reth-cli-runner`, without pulling `reth` in as a dependency.

## Features

- `rayon`: Enable the mirrored rayon-backed worker pools and ordered parallel iteration helpers
- `reth-tasks`: Re-export upstream `reth_tasks` instead of the local mirror when exact type identity is needed
