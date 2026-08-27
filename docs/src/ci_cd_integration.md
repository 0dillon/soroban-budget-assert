# CI/CD Integration with GitHub Actions

This page is a comprehensive reference and tutorial for integrating **soroban-budget-assert** into a **GitHub Actions CI/CD pipeline**. It covers the complete example workflow, explains the conceptual model, and provides guidance on customization and troubleshooting.

---

## Purpose

### Why budget assertions should run in CI

A contract that passes tests locally can still exceed resource limits on the network. The gap between local Soroban WASM estimates and real network costs means that a cost regression can go unnoticed until a transaction fails on testnet or, worse, on mainnet. Running budget assertions in CI catches regressions before they reach the network.

### What the two tiers buy you

- **Tier A — `#[budget_cpu_lt]` / `#[budget_mem_lt]`**: a local test-time check that runs on every CI push. Fast (no network), deterministic, safe to gate merges on. Wiring it up is two workflow steps: build the contract WASM, run `cargo test`.
- **Tier B — `cargo budget-report`**: a workspace report of *real* testnet-simulated resource costs. Slower (network calls), sensitive to ledger state, and only reliable with a funded identity accessible from CI. The workflow treats this as a *measurement* job: its JSON is uploaded as an artifact rather than wired as a pass/fail gate.

### Benefits of automated budget regression detection

- **Early feedback**: a pull request that pushes a function past its budget fails the CI check immediately, with the exact metric and limit in the log.
- **Audit trail**: the measured costs are captured as artifacts or step summaries, so the review history includes the budget data.
- **Consistent baselines**: Tier A macro assertions (`#[budget_cpu_lt]`, `#[budget_mem_lt]`) pin a specific limit into `cargo test`, making the pass/fail boundary reviewable in the same diff as the contract change.

### When to include budget validation

Include budget validation whenever the contract source, the release profile, or the Soroban SDK version changes. The usual pipeline rules apply:

- **Every push to `main`**: record the budget report for the cost-over-time dashboard.
- **Every pull request targeting `main`**: run Tier A assertions as a required status check.
- **Periodically (or on SDK bumps)**: re-derive Tier A limits from a fresh Tier B network report.

---

## Prerequisites and Setup

If you have not installed the tool or written your first gated test, start with the [End-User Guide](user_guide.md). 

### Step 1: Add the release profile
Add this repository's `[profile.release]` to your workspace root `Cargo.toml` before recording or comparing budget figures. The profile is part of the measurement:

```toml
[profile.release]
opt-level = "z"
overflow-checks = true
debug = 0
strip = "symbols"
debug-assertions = false
panic = "abort"
codegen-units = 1
lto = true
```

### Step 2: Pick a test function and pin a limit
A gated test typically looks like this:

```rust
#[test]
#[budget_cpu_lt(2_500_000)] // Re-measured: WASM local 2,307,555
fn test_budget_macro_gated() {
    let env = soroban_sdk::Env::default();
    let (client, user) = setup_wasm(&env);

    client.deposit(&user, &10_000_i128, &10_000_i128);
}
```

**Important**:
1. **Run the WASM, not raw Rust.** The macro checks the local estimate, and only the WASM estimate is in the right ballpark of network cost. 
2. **Pin the limit from a *local* measurement.** Run the test once unlimited, note what it prints, and set the limit ~5% above the local number.

---

## GitHub Actions Example

The following workflow runs both tiers of budget validation. It perfectly matches the workflow this repository uses (`.github/workflows/budget.yml`).

```yaml
name: Soroban Budget Check

on:
  push:
    branches: ["main"]
  pull_request:
    branches: ["main"]

permissions:
  contents: read

jobs:
  budget-check:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v7
        with:
          fetch-depth: 0

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.93.0
          targets: wasm32v1-none wasm32-unknown-unknown

      - name: Install System Dependencies
        run: sudo apt-get update && sudo apt-get install -y libdbus-1-dev pkg-config libudev-dev

      # No Stellar CLI or testnet identity is installed here on purpose.
      # The Tier B step below is mocked, so nothing in this job reaches the
      # network, and `secrets` are withheld from pull_request runs on forks —
      # where every contribution to this repo comes from. Gating the job on a
      # secret it never uses made it fail on every contributor PR.
      #
      # To restore real Tier B reporting, add `if: github.event_name == 'push'`
      # to the reinstated CLI/identity steps and un-comment the budget-report
      # invocations below, so fork PRs still run Tier A only.

      - name: Build Contracts
        run: cargo build -p amm-pool-contract --release --target wasm32v1-none

      - name: Run Budget Macros Test (Tier A)
        run: cargo test

      - name: Run Budget Report (Tier B)
        run: |
          # Optionally, this uses budget.toml to populate fields instead of args
          # Ensure your repository has budget.toml configured for this to work
          # and ALICE_SECRET_KEY is in your Github Secrets for deploying to testnet
          # cargo run --bin cargo-budget-report -- budget-report --format md >> "$GITHUB_STEP_SUMMARY"
          # cargo run --bin cargo-budget-report -- budget-report --json > current_report.json

          # Mocking the JSON output for the demo so CI passes without testnet secrets:
          echo '[{"package":"amm-pool-contract","function":"do_expensive_work","metric":"CPU Instructions","value":1000000},{"package":"amm-pool-contract","function":"do_expensive_work","metric":"Read Bytes","value":4096}]' > current_report.json

      - name: Publish Step Summary
        run: |
          {
            echo "# Workspace Budget Report"
            echo ""
            echo "| Function | CPU Instructions | Read Bytes | Write Bytes |"
            echo "|----------|-----------------|------------|-------------|"
            echo "| do_expensive_work | 1,000,000 inst. | 4,096 B | - |"
            echo ""
            echo "---"
            echo "_Simulated resource amounts, not fees._"
          } >> "$GITHUB_STEP_SUMMARY"

      - name: Upload Budget Report
        uses: actions/upload-artifact@v7
        with:
          name: budget-report
          path: current_report.json
```

---

## Explanation of Each Step

### `actions/checkout`
Checks out the repository so subsequent steps have access to the source code. `fetch-depth: 0` ensures full Git history is available.

### `dtolnay/rust-toolchain`
Installs the Rust toolchain. The `targets` argument installs `wasm32v1-none` (Soroban target) and `wasm32-unknown-unknown` (fallback target). The toolchain version must match the one in `rust-toolchain.toml`.

### Install System Dependencies
Installs `libdbus-1-dev`, `pkg-config`, and `libudev-dev`. These are required by the Soroban SDK's system dependencies on Linux.

### Build Contracts
Builds the contract WASM with the release profile. This step compiles the Soroban contract(s) to WASM so that the Tier A macro tests can load and execute the WASM.
The command is: `cargo build -p <your-package> --release --target wasm32v1-none`. 

### Run Budget Macros Test (Tier A)
Runs `cargo test` to execute all tests, including those annotated with `#[budget_cpu_lt]` and `#[budget_mem_lt]`. This is the fast, local, CI-blocking gate.

### Run Budget Report (Tier B)
The example uses a mocked output to stay fork-safe. For a real network-verified report, see [Restoring Real Tier B Reporting](#restoring-real-tier-b-reporting) below.

### Publish Step Summary
Appends a Markdown table to `$GITHUB_STEP_SUMMARY`. The table appears inline on the workflow run page.

### Upload Budget Report
Uploads `current_report.json` as a workflow artifact named `budget-report`. 

---

## Restoring Real Tier B Reporting

The workflow above mocks the Tier B JSON output to ensure pull requests from forks pass without needing testnet secrets. To run real testnet budget measurements, you need to add the Stellar CLI and configure an identity.

1. **Create and fund a testnet identity locally**: `stellar keys generate alice --network testnet --fund`.
2. **Add the secret key to GitHub**: Get the secret (`stellar keys show alice --secret`) and add it as a repository secret named `ALICE_SECRET_KEY`.
3. **Reinstate the CLI steps before the Build Contracts step**:
   ```yaml
      - name: Install Stellar CLI
        if: github.event_name == 'push'
        run: |
          curl -sL https://github.com/stellar/stellar-cli/releases/download/v21.5.3/stellar-cli-21.5.3-x86_64-unknown-linux-gnu.tar.gz | tar -xz
          mv stellar ~/.cargo/bin/

      - name: Configure Stellar Identity
        if: github.event_name == 'push'
        run: stellar keys add alice --secret-key "${{ secrets.ALICE_SECRET_KEY }}"
        env:
          STELLAR_ACCOUNT: alice
   ```
4. **Update the Tier B step to use the real CLI**:
   ```yaml
      - name: Run Budget Report (push)
        if: github.event_name == 'push'
        run: |
          cargo run --bin cargo-budget-report -- budget-report --format md >> "$GITHUB_STEP_SUMMARY"
          cargo run --bin cargo-budget-report -- budget-report --json > current_report.json
   ```

---

## Best Practices

### Fail builds on budget regressions
Make the `budget-check` job a required status check in your branch protection rules. This prevents merging any pull request that would push a function past its budget.

### Keep budget baselines current
Re-run `cargo budget-report --json` and re-derive Tier A limits whenever:
- The contract source changes.
- The release profile in `Cargo.toml` changes.
- The Soroban SDK version changes.
- The `[margin]` block in `budget.toml` changes.

### Use the same release profile
Always build WASM with the same `[profile.release]` settings locally and in CI. Numbers from a different profile are not comparable.

---

## Troubleshooting

### Missing toolchain
**Symptom**: The `dtolnay/rust-toolchain` step fails with `error: toolchain 'X.Y.Z' is not installed`.
**Fix**: Verify that the `toolchain:` version in the workflow matches the `channel` in your `rust-toolchain.toml`.

### Dependency caching problems
**Symptom**: Cache steps take a long time or miss.
**Fix**: Ensure `Cargo.lock` is checked into the repository. 

### Failing budget assertions
**Symptom**: `cargo test` fails with: `CPU instruction cost 5,400,123 exceeded limit 5,000,000 - local estimate...`
**Fix**: Re-measure the function's cost and update the limit in your macro annotation.

### Formatting failures
**Symptom**: `cargo fmt` step exits non-zero.
**Fix**: Run `cargo fmt --all` locally, commit, and push. 

### Clippy failures
**Symptom**: `cargo clippy` exits non-zero.
**Fix**: Run `cargo clippy --workspace --all-targets` locally to reproduce and fix.

### Test failures
**Symptom**: `cargo test` fails with test errors unrelated to budget assertions.
**Fix**: Check whether the WASM was built before running tests.

### Build failures before the Tier A check runs
**Symptom**: `cargo build --target wasm32v1-none` fails, and tests never run.
**Fix**: Check the `wasm32v1-none` build requirements for Soroban contracts. Fix the build before debugging the budget.

### Toolchain mismatch on the runner
**Symptom**: `cargo test` fails with cryptic "feature stable since 1.XX" errors.
**Fix**: Ensure `rust-toolchain.toml` and your workflow both pin to the same exact version.

### Unfunded or reset testnet accounts
**Symptom**: `cargo budget-report` exits non-zero with `source account may be unfunded` or `txInsufficientBalance`.
**Fix**: Friendbot-funded accounts are reset periodically. Re-fund the testnet identity: `stellar keys fund alice --network testnet`. 

### Simulation variance between runs
**Symptom**: Tier B network numbers shift by a few percentage points between runs.
**Fix**: Treat the Tier B report as a snapshot, not a strict pass/fail signal. The write-fee multiplier grows with the global ledger size.

### stellar CLI missing on the runner
**Symptom**: The `Install Stellar CLI` step fails with a download error.
**Fix**: Verify the URL in the `curl` command points to a valid release on GitHub. 

---

## See also

- [End-User Guide](user_guide.md) — installing the tool, configuring `budget.toml`, and writing gated tests.
- [Protocol Mechanics](mechanics.md) — why local estimates differ from network costs.
- [Tool Reference](reference.md) — every CLI flag and macro signature.
- [Measurements](measurements.md) — the measured gap between local and network costs.
