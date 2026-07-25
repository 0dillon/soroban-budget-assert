#![cfg(test)]

use amm_pool_contract::{ConstantProductPool, ConstantProductPoolClient};
use budget_macros::{budget_cpu_lt, budget_mem_lt};
use soroban_sdk::{testutils::Address as _, Address, Env};

/// A Drop guard that writes `budget.json` on creation and removes it on drop
/// (including during stack unwinding from a panic).
struct BudgetJsonGuard;

impl BudgetJsonGuard {
    fn create(content: &str) -> Self {
        std::fs::write("budget.json", content).expect("failed to write budget.json");
        BudgetJsonGuard
    }
}

impl Drop for BudgetJsonGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file("budget.json");
    }
}

fn setup_wasm(env: &Env) -> (ConstantProductPoolClient<'_>, Address) {
    let wasm_path = "../target/wasm32-unknown-unknown/release/amm_pool_contract.wasm";
    let wasm = std::fs::read(wasm_path).expect("WASM file not found, did you run cargo build?");
    #[allow(deprecated)]
    let contract_id = env.register_contract_wasm(None, wasm.as_slice());
    let client = ConstantProductPoolClient::new(env, &contract_id);

    let user = Address::generate(env);

    client.initialize();

    env.mock_all_auths();

    env.cost_estimate().budget().reset_unlimited();

    (client, user)
}

#[test]
fn test_budget_raw_rust() {
    let env = Env::default();
    let contract_id = env.register(ConstantProductPool, ());
    let client = ConstantProductPoolClient::new(&env, &contract_id);

    env.cost_estimate().budget().reset_unlimited();

    client.do_expensive_work(&10_000);

    let budget = env.cost_estimate().budget();
    println!("=== RAW RUST LOCAL ===");
    println!("CPU instructions: {}", budget.cpu_instruction_cost());
    println!("Memory bytes: {}", budget.memory_bytes_cost());
}

#[test]
fn test_budget_wasm() {
    let env = Env::default();
    let (client, user) = setup_wasm(&env);

    client.deposit(&user, &10_000_i128, &10_000_i128);
    client.swap(&user, &true, &100_i128, &90_i128);
    client.withdraw(&user, &1_000_i128, &900_i128, &900_i128);

    let budget = env.cost_estimate().budget();
    println!("=== WASM LOCAL ===");
    println!("CPU instructions: {}", budget.cpu_instruction_cost());
    println!("Memory bytes: {}", budget.memory_bytes_cost());
}

#[test]
#[budget_cpu_lt(2500000)] // Re-measured: WASM local 2307555, simulates deposit+swap+withdraw
fn test_budget_macro_gated() {
    let env = Env::default();
    let (client, user) = setup_wasm(&env);

    client.deposit(&user, &10_000_i128, &10_000_i128);
    client.swap(&user, &true, &100_i128, &90_i128);
    client.withdraw(&user, &1_000_i128, &900_i128, &900_i128);
}

#[test]
#[should_panic(
    expected = "local estimate, real network cost may differ significantly in either direction"
)]
#[budget_cpu_lt(1000000)] // Deliberate regression: AMM pool costs ~2.3M CPU
fn test_budget_macro_deliberate_regression() {
    let env = Env::default();
    let (client, user) = setup_wasm(&env);

    client.deposit(&user, &10_000_i128, &10_000_i128);
    client.swap(&user, &true, &100_i128, &90_i128);
    client.withdraw(&user, &1_000_i128, &900_i128, &900_i128);
}

#[test]
#[should_panic(
    expected = "local estimate, real network cost may differ significantly in either direction"
)]
#[budget_mem_lt(1)] // Deliberate regression: any real memory cost exceeds an impossible 1-byte limit
fn test_budget_macro_mem_deliberate_regression() {
    let env = Env::default();
    let (client, user) = setup_wasm(&env);

    client.deposit(&user, &10_000_i128, &10_000_i128);
    client.swap(&user, &true, &100_i128, &90_i128);
    client.withdraw(&user, &1_000_i128, &900_i128, &900_i128);
}

#[test]
#[budget_cpu_lt(env = "TEST_MAX_CPU")]
fn test_budget_macro_dynamic_env() {
    let budget_env_resolve = |var: &str| -> Option<String> {
        if var == "TEST_MAX_CPU" {
            Some("2500000".to_string())
        } else {
            None
        }
    };
    let env = Env::default();
    let (client, user) = setup_wasm(&env);

    client.deposit(&user, &10_000_i128, &10_000_i128);
    client.swap(&user, &true, &100_i128, &90_i128);
    client.withdraw(&user, &1_000_i128, &900_i128, &900_i128);
}

#[test]
#[budget_cpu_lt(env = "TEST_MAX_CPU_FALLBACK")]
fn test_budget_macro_dynamic_env_fallback() {
    let budget_env_resolve = |_var: &str| -> Option<String> { None };
    let env = Env::default();
    let (client, user) = setup_wasm(&env);

    client.deposit(&user, &10_000_i128, &10_000_i128);
    client.swap(&user, &true, &100_i128, &90_i128);
    client.withdraw(&user, &1_000_i128, &900_i128, &900_i128);
}

// ---------------------------------------------------------------------------
// JSON config tests
// ---------------------------------------------------------------------------

#[test]
#[budget_cpu_lt(config = "cpu_instructions")]
fn test_budget_macro_json_config_valid() {
    // The BudgetJsonGuard writes budget.json before the test body runs.
    // The macro assertion runs AFTER all statements in the test body, but
    // the guard's Drop (which removes the file) runs when the scope exits
    // — which is after the assertion. Additionally, Drop runs during stack
    // unwinding if the assertion panics, ensuring cleanup.
    let _guard = BudgetJsonGuard::create(r#"{"cpu_instructions": 2500000}"#);
    let env = Env::default();
    let (client, user) = setup_wasm(&env);

    client.deposit(&user, &10_000_i128, &10_000_i128);
    client.swap(&user, &true, &100_i128, &90_i128);
    client.withdraw(&user, &1_000_i128, &900_i128, &900_i128);
}

#[test]
#[budget_mem_lt(config = "memory_bytes")]
fn test_budget_macro_json_config_mem_valid() {
    let _guard = BudgetJsonGuard::create(r#"{"memory_bytes": 1000000}"#);
    let env = Env::default();
    let (client, user) = setup_wasm(&env);

    client.deposit(&user, &10_000_i128, &10_000_i128);
    client.swap(&user, &true, &100_i128, &90_i128);
    client.withdraw(&user, &1_000_i128, &900_i128, &900_i128);
}

#[test]
#[budget_cpu_lt(config = "non_existent_key")]
fn test_budget_macro_json_config_missing_key() {
    // A valid json file but without the requested key should fall back to
    // u64::MAX and allow the assertion to pass.
    let _guard = BudgetJsonGuard::create(r#"{"some_other_key": 100}"#);
    let env = Env::default();
    let (client, user) = setup_wasm(&env);

    client.deposit(&user, &10_000_i128, &10_000_i128);
    client.swap(&user, &true, &100_i128, &90_i128);
    client.withdraw(&user, &1_000_i128, &900_i128, &900_i128);
}

#[test]
#[should_panic(
    expected = "local estimate, real network cost may differ significantly in either direction"
)]
#[budget_cpu_lt(config = "cpu_instructions_deliberate")]
fn test_budget_macro_json_config_deliberate_regression() {
    // A deliberately low threshold in the config file should trigger a panic
    // just like the hard-coded regression test.
    let _guard = BudgetJsonGuard::create(r#"{"cpu_instructions_deliberate": 1}"#);
    let env = Env::default();
    let (client, user) = setup_wasm(&env);

    client.deposit(&user, &10_000_i128, &10_000_i128);
    client.swap(&user, &true, &100_i128, &90_i128);
    client.withdraw(&user, &1_000_i128, &900_i128, &900_i128);
}

#[test]
#[budget_cpu_lt(config = "cpu_instructions")]
fn test_budget_macro_json_config_fallback_no_file() {
    // When no budget.json exists at all, the macro falls back to u64::MAX
    // and the assertion passes.
    let env = Env::default();
    let (client, user) = setup_wasm(&env);

    client.deposit(&user, &10_000_i128, &10_000_i128);
    client.swap(&user, &true, &100_i128, &90_i128);
    client.withdraw(&user, &1_000_i128, &900_i128, &900_i128);
}

#[test]
#[budget_cpu_lt(config = "cpu_instructions")]
fn test_budget_macro_json_config_invalid_json() {
    // Malformed JSON content should cause parse_config_value to return None,
    // falling back to u64::MAX so the assertion passes.
    let _guard = BudgetJsonGuard::create("this is not valid json at all");
    let env = Env::default();
    let (client, user) = setup_wasm(&env);

    client.deposit(&user, &10_000_i128, &10_000_i128);
    client.swap(&user, &true, &100_i128, &90_i128);
    client.withdraw(&user, &1_000_i128, &900_i128, &900_i128);
#[test]
#[should_panic(expected = "budget_cpu_lt: env var BAD_CPU_LIMIT")]
#[budget_cpu_lt(env = "BAD_CPU_LIMIT")]
fn test_budget_macro_dynamic_env_invalid_value() {
    let budget_env_resolve = |var: &str| -> Option<String> {
        if var == "BAD_CPU_LIMIT" {
            Some("1_000_000".to_string())
        } else {
            None
        }
    };
    let env = Env::default();
    let contract_id = env.register(ConstantProductPool, ());
    let client = ConstantProductPoolClient::new(&env, &contract_id);
    env.cost_estimate().budget().reset_unlimited();
    client.do_expensive_work(&10_000);
}

#[test]
#[should_panic(expected = "budget_mem_lt: env var BAD_MEM_LIMIT")]
#[budget_mem_lt(env = "BAD_MEM_LIMIT")]
fn test_budget_macro_mem_dynamic_env_invalid_value() {
    let budget_env_resolve = |var: &str| -> Option<String> {
        if var == "BAD_MEM_LIMIT" {
            Some("not_a_number".to_string())
        } else {
            None
        }
    };
    let env = Env::default();
    let contract_id = env.register(ConstantProductPool, ());
    let client = ConstantProductPoolClient::new(&env, &contract_id);
    env.cost_estimate().budget().reset_unlimited();
    client.do_expensive_work(&10_000);
}
