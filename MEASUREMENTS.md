# Measurements

This file records empirical cost measurements comparing local Soroban budget estimates against real network costs. Every measurement PR adds its numbers here so the series stays comparable and in one place.

## Methodology

Each measurement compares a local budget estimate against a network-verified figure for the same operation. The local estimate comes from `Env::cost_estimate().budget()` in a test that registers the contract as WASM with `register_contract_wasm` (except where noted). The network figure comes from `simulateTransaction` on Soroban testnet — the same endpoint the network uses to charge non-refundable resource costs.

The WASM is compiled with the profile specified in the **Build profile** column. The direction of the local-vs-network gap is not stable across profiles; the same contract built with Cargo's default release profile can produce a gap pointing in the opposite direction of one built with the size-optimization profile. Every figure includes its build context.

For the storage-write measurement, the complete capture record is checked in at [`cargo-budget-report/fixtures/storage_write_benchmark.json`](cargo-budget-report/fixtures/storage_write_benchmark.json). It records the fixture arguments, local capture command, network capture method, both figures, and the calculated delta.

### Column reference

| Column | Meaning |
|---|---|
| **Operation type** | Category of operation being measured |
| **Local estimate** | Value reported by `Env::cost_estimate().budget()` in a WASM-registered local test |
| **Network figure** | Value returned by `simulateTransaction` on Soroban testnet (ground truth) |
| **Delta** | (local − network) / network, expressed as a percentage; positive means local overestimates |
| **Fixture** | Contract, function, and arguments used for the measurement |
| **Build profile** | Cargo profile used to compile the WASM |
| **Toolchain** | Rust toolchain version (`rustc --version`) |
| **Date** | Date the measurement was taken |

## Existing measurements

These figures were produced during the initial tool development and are published in the [Protocol Mechanics documentation](docs/src/mechanics.md). They serve as the worked example for contributors adding new measurements.

### CPU instructions

| Operation type | Local estimate | Network figure | Delta | Fixture | Build profile | Toolchain | Date |
|---|---:|---:|---:|---|---|---|---|
| Mixed compute + storage (native Rust) | 143,887 | 756,678 | −81.0% | `amm-pool-contract::do_expensive_work(10_000)` | N/A (native test, no WASM) | rustc 1.81 | 2025-Q1 |
| Mixed compute + storage (WASM) | 901,816 | 756,678 | +19.2% | `amm-pool-contract::do_expensive_work(10_000)` | size-opt (`opt-level="z"`, LTO, `codegen-units=1`) | rustc 1.81 | 2025-Q1 |
| Mixed compute + storage (WASM) | 767,049 | 832,006 | −7.8% | `amm-pool-contract::do_expensive_work(10_000)` | default `release` (`opt-level=3`) | rustc 1.81 | 2025-Q1 |
| Storage write (WASM) | 36,840 | 44,512 | −17.2% | `amm-pool-contract::write_bytes(1,024 bytes)` | size-opt (`opt-level="z"`, LTO, `codegen-units=1`) | rustc 1.81 | 2026-07-26 |

The native Rust row is included solely to illustrate that native estimates are unreliable for budget decisions. Only WASM-mode estimates should be used for assertions.

The first three rows measure the same `do_expensive_work(10_000)` function, which mixes a compute loop (`n` iterations of `wrapping_add(wrapping_mul)`) with a storage write (`Vec` of up to 100 elements written to `env.storage().instance().set`). The numbers are aggregate costs of both operations.

The storage-write row isolates the `write_bytes` fixture with a 1,024-byte value. Its delta is calculated as `(36,840 − 44,512) / 44,512 = −0.1724`, so the WASM-registered local estimate is 17.2% lower than the testnet simulation for this operation and underestimates the network cost.

## Unmeasured operation types

The following operation types have open measurement issues and no published figures yet. When adding a measurement, follow the column format above and include the build profile and toolchain.

| Operation type | Issue | Status |
|---|---|---|
| Host-function-call operations | [#86](https://github.com/Tollcraft/soroban-budget-assert/issues/86) | Open |
| VM-instruction-heavy operations | [#87](https://github.com/Tollcraft/soroban-budget-assert/issues/87) | Open |
| Memory bytes | [#122](https://github.com/Tollcraft/soroban-budget-assert/issues/122) | Open |
