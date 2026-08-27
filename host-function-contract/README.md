# Host-Function Call Measurement Fixture (`host-function-contract`)

`host-function-contract` is a dedicated benchmark fixture crate within this repository. Its primary purpose is to isolate and measure the budget cost of repeated Soroban host-function invocations (specifically `env.ledger().sequence()`) without introducing storage reads, storage writes, or complex arithmetic computation.

## Why This Crate Exists

Soroban smart contracts incur budget charges both from WASM instruction execution and from host-function calls. To establish accurate Tier A and Tier B budget margins, we must measure the local-vs-network cost gap for isolated operation types:

- **AMM Pool Contract (`amm-pool-contract`):** Measures mixed compute + storage, storage writes, storage reads, memory allocations, and authorization calls.
- **Host Function Contract (`host-function-contract`):** Isolates repeated host-function call overhead (`env.ledger().sequence()`) with zero storage or side effects.

By isolating host-function calls in a zero-storage fixture, measurements reflect only the host-function invocation cost and WASM loop mechanics.

## Benchmark Operations

The core benchmark fixture exposes several single-purpose functions, each looping `iterations` times over one host function with no storage, event, or arithmetic side-effects:

```rust
HostFunctionBenchmark::repeated_sequence(env: Env, iterations: u32) -> u32     // env.ledger().sequence()
HostFunctionBenchmark::repeated_timestamp(env: Env, iterations: u32) -> u64     // env.ledger().timestamp()
HostFunctionBenchmark::repeated_hash(env: Env, iterations: u32) -> u32          // env.crypto().sha256
HostFunctionBenchmark::repeated_bytes_new(env: Env, iterations: u32) -> u32     // Bytes::new
```

Measuring four distinct host functions in the same module tests whether the local-vs-network CPU gap varies by host function — it does (see `MEASUREMENTS.md`).

## Workspace Membership

> **Note on Workspace Membership:**  
> This crate was originally omitted from `[workspace.members]` in the root `Cargo.toml` due to an oversight during initial fixture setup. It has since been formally added to `workspace.members`. It is a standard workspace member, ensuring it is included in workspace builds (`cargo build`), workspace test runs (`cargo test --workspace`), formatting (`cargo fmt`), and static analysis (`cargo clippy`).

## How to Build and Run

### Building from a Clean Checkout

To compile the contract to WASM target:

```bash
cargo build -p host-function-contract --target wasm32-unknown-unknown --release
```

Or for newer Soroban toolchain targets (`wasm32v1-none`):

```bash
cargo build -p host-function-contract --target wasm32v1-none --release
```

### Running Tests

To run the package unit tests:

```bash
cargo test -p host-function-contract
```

To run all workspace tests including this crate:

```bash
cargo test --workspace
```

## Measurement Data and Reproducing Results

This fixture is used to record figures in [`MEASUREMENTS.md`](../MEASUREMENTS.md) and [`docs/src/measurements.md`](../docs/src/measurements.md).

### Local Budget Estimate

Run the measurement test to capture local WASM estimates per function (values below are for 1,000 iterations under rustc 1.91.0 / soroban-sdk 27):

```
cargo build -p host-function-contract --target wasm32v1-none --release
cargo test -p host-function-contract --test measure_host_fn_gap -- --nocapture
```

- **`repeated_sequence(1_000)`**: 1,759,859 CPU instructions (1,239,673 mem bytes)
- **`repeated_timestamp(1_000)`**: 3,861,391 CPU instructions (1,239,673 mem bytes)
- **`repeated_bytes_new(1_000)`**: 2,405,859 CPU instructions (1,343,673 mem bytes)
- **`repeated_hash(1_000)`**: 7,488,773 CPU instructions (1,391,800 mem bytes)

### Network Simulation Figure

Deploy the compiled WASM to Soroban testnet and submit a `simulateTransaction` for each function (decode the `resources.instructions` field of `transactionData`):

- **`repeated_sequence(1_000)`**: 2,194,275 CPU instructions
- **`repeated_timestamp(1_000)`**: 4,379,869 CPU instructions
- **`repeated_bytes_new(1_000)`**: 2,865,075 CPU instructions
- **`repeated_hash(1_000)`**: 8,042,983 CPU instructions

### Calculated Gaps (Deltas)

$$\text{Delta} = \frac{\text{Local} - \text{Network}}{\text{Network}}$$

| Function | Local | Network | Delta |
|---|---:|---:|---:|
| `repeated_sequence(1_000)` | 1,759,859 | 2,194,275 | −19.8% |
| `repeated_timestamp(1_000)` | 3,861,391 | 4,379,869 | −11.8% |
| `repeated_bytes_new(1_000)` | 2,405,859 | 2,865,075 | −16.0% |
| `repeated_hash(1_000)` | 7,488,773 | 8,042,983 | −6.9% |

All four host functions are underestimated locally, so a Tier A margin derived from the local estimate would under-budget against real on-chain execution for each of them. The gap varies by function (6.9%–19.8%), so no single local figure represents the whole host-function category. See [`MEASUREMENTS.md`](../MEASUREMENTS.md) for the full methodology, gap-stability analysis across call counts, and reproduction steps.

