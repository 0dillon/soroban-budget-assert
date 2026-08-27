//! # Host-Function Benchmark Fixture (`host-function-contract`)
//!
//! This crate provides a Soroban smart contract fixture designed specifically for
//! empirical cost measurement and baseline gap analysis of repeated host-function invocations.
//!
//! ## Purpose
//!
//! The contract measures repeated calls to `env.ledger().sequence()` without introducing
//! contract storage state, event logging, or CPU-intensive math loops. This isolates the
//! host-function overhead from other billing dimensions (such as read/write bytes or VM instructions).
//!
//! See `README.md` and `MEASUREMENTS.md` at the repository root for detailed methodology
//! and captured figures.

#![no_std]

use soroban_sdk::{contract, contractimpl, Bytes, Env};

/// Benchmark contract fixture for measuring the gap between local budget estimates
/// and live network simulation figures for repeated host-function calls.
#[contract]
pub struct HostFunctionBenchmark;

#[contractimpl]
impl HostFunctionBenchmark {
    /// Calls the `env.ledger().sequence()` host function repeatedly for `iterations` count
    /// and returns the final sequence value.
    ///
    /// Returning the sequence value prevents the compiler from optimizing out the loop
    /// while keeping the execution entirely free of storage side-effects.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment instance.
    /// * `iterations` - Number of times to invoke `env.ledger().sequence()`.
    pub fn repeated_sequence(env: Env, iterations: u32) -> u32 {
        let mut sequence = 0;

        for _ in 0..iterations {
            sequence = env.ledger().sequence();
        }

        sequence
    }

    /// Calls `env.ledger().timestamp()` repeatedly for `iterations` count
    /// and returns the final timestamp value.
    ///
    /// This isolates the cost of a different ledger-read host function from
    /// `repeated_sequence` to test whether the local-vs-network gap varies
    /// across distinct host functions within the same module.
    pub fn repeated_timestamp(env: Env, iterations: u32) -> u64 {
        let mut timestamp: u64 = 0;

        for _ in 0..iterations {
            timestamp = env.ledger().timestamp();
        }

        timestamp
    }

    /// Hashes a small input buffer with SHA-256 for `iterations` count and
    /// returns the number of iterations completed.
    ///
    /// Each iteration allocates an 8-byte `Bytes` value and passes it through
    /// `env.crypto().sha256()`, exercising the cryptographic host function
    /// category. The return value prevents dead-code elimination while
    /// keeping the function free of storage side-effects.
    pub fn repeated_hash(env: Env, iterations: u32) -> u32 {
        let input = Bytes::from_slice(&env, b"hashben");

        for _ in 0..iterations {
            let _digest = env.crypto().sha256(&input);
        }

        iterations
    }

    /// Creates `iterations` fresh `Bytes` values via `Bytes::new(&env)` and
    /// returns the count completed.
    ///
    /// Each iteration exercises the Bytes-allocation host function without
    /// any storage, event, or cryptographic side-effects, isolating the
    /// per-call cost of the Bytes constructor.
    pub fn repeated_bytes_new(env: Env, iterations: u32) -> u32 {
        for _ in 0..iterations {
            let _b = Bytes::new(&env);
        }

        iterations
    }
}
