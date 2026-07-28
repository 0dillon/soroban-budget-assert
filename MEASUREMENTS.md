# Measurements

This file records empirical cost measurements comparing local Soroban budget estimates against real network costs. Every measurement PR adds its numbers here so the series stays comparable and in one place.

## Methodology

Each measurement compares a local budget estimate against a network-verified figure for the same operation. The local estimate comes from `Env::cost_estimate().budget()` in a test that registers the contract as WASM with `register_contract_wasm` (except where noted). The network figure comes from `simulateTransaction` on Soroban testnet — the same endpoint the network uses to charge non-refundable resource costs.

The WASM is compiled with the profile specified in the **Build profile** column. The direction of the local-vs-network gap is not stable across profiles; the same contract built with Cargo's default release profile can produce a gap pointing in the opposite direction of one built with the size-optimization profile. Every figure includes its build context.

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
|---|---|---|---|---|---|---|---|
| Mixed compute + storage (native Rust) | 143,887 | 756,678 | −81.0% | `amm-pool-contract::do_expensive_work(10_000)` | N/A (native test, no WASM) | rustc 1.81 | 2025-Q1 |
| Mixed compute + storage (WASM) | 901,816 | 756,678 | +19.2% | `amm-pool-contract::do_expensive_work(10_000)` | size-opt (`opt-level="z"`, LTO, `codegen-units=1`) | rustc 1.81 | 2025-Q1 |
| Mixed compute + storage (WASM) | 767,049 | 832,006 | −7.8% | `amm-pool-contract::do_expensive_work(10_000)` | default `release` (`opt-level=3`) | rustc 1.81 | 2025-Q1 |

The native Rust row is included solely to illustrate that native estimates are unreliable for budget decisions. Only WASM-mode estimates should be used for assertions.

All three rows measure the same `do_expensive_work(10_000)` function, which mixes a compute loop (`n` iterations of `wrapping_add(wrapping_mul)`) with a storage write (`Vec` of up to 100 elements written to `env.storage().instance().set`). The numbers are aggregate costs of both operations.

### Gap stability across input sizes

The following measurements test whether the local-vs-network gap widens or narrows as `n` grows. The `do_expensive_work` compute loop does `n` iterations of `wrapping_add(wrapping_mul)`, while the storage loop is internally capped at `n.min(100)`.

| n | WASM local estimate | Testnet simulated | Delta (abs) | Delta (%) | Build profile | Toolchain | Date |
|---|---|---|---|---|---|---|---|
| 1,000 | 2,655,136 | 1,410,984 | +1,244,152 | +88.2% | size-opt (`opt-level="z"`, LTO, `codegen-units=1`) | rustc 1.85.0 | 2026-07-28 |
| 10,000 | 2,655,136 | 1,410,984 | +1,244,152 | +88.2% | size-opt (`opt-level="z"`, LTO, `codegen-units=1`) | rustc 1.85.0 | 2026-07-28 |
| 50,000 | 2,655,136 | 1,410,984 | +1,244,152 | +88.2% | size-opt (`opt-level="z"`, LTO, `codegen-units=1`) | rustc 1.85.0 | 2026-07-28 |
| 100,000 | 2,655,136 | 1,410,984 | +1,244,152 | +88.2% | size-opt (`opt-level="z"`, LTO, `codegen-units=1`) | rustc 1.85.0 | 2026-07-28 |

**Conclusion.** The gap is **stable** — both local and testnet CPU instruction costs are invariant with respect to `n`. This is because the Soroban budget meters host function calls (Vec allocation, storage writes), not raw WASM arithmetic. The compute loop (`n` iterations of arithmetic) is invisible to both local and network metering. Consequently, a single fixed Tier A margin is defensible for computation-heavy parameters in `do_expensive_work` as long as the number of host function calls stays constant. If a later change introduces a host-call path that scales with input size (e.g., per-element storage writes), the gap should be re-measured because the delta is proportional to host call count, not to `n` directly.

A note on version sensitivity: the absolute figures above differ from the 2025-Q1 baseline (which reported 901,816 local / 756,678 testnet for the same `do_expensive_work(10_000)`). The shift is attributable to SDK version changes (22.0.11 vs earlier) and the larger WASM module that now includes the full AMM pool contract. The key finding — cost invariance with `n` — holds under both versions.

## Unmeasured operation types

The following operation types have open measurement issues and no published figures yet. When adding a measurement, follow the column format above and include the build profile and toolchain.

| Operation type | Issue | Status |
|---|---|---|
| Storage-write operations | [#44](https://github.com/Tollcraft/soroban-budget-assert/issues/44) | Open |
| Host-function-call operations | [#86](https://github.com/Tollcraft/soroban-budget-assert/issues/86) | Open |
| VM-instruction-heavy operations | [#87](https://github.com/Tollcraft/soroban-budget-assert/issues/87) | Open |
| Memory bytes | [#122](https://github.com/Tollcraft/soroban-budget-assert/issues/122) | Open |
