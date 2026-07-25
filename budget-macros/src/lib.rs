extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse::ParseStream, Ident, ItemFn, LitInt, LitStr, Token};

#[derive(Clone)]
enum BudgetLimit {
    Int(u64),
    EnvVar(String),
}

impl Parse for BudgetLimit {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) {
            let ident: Ident = input.parse()?;
            if ident != "env" {
                return Err(syn::Error::new(ident.span(), "expected `env`"));
            }
            if input.peek(syn::token::Paren) {
                // env("VAR")
                let content;
                syn::parenthesized!(content in input);
                let lit: LitStr = content.parse()?;
                Ok(BudgetLimit::EnvVar(lit.value()))
            } else {
                // env = "VAR"
                input.parse::<Token![=]>()?;
                let lit: LitStr = input.parse()?;
                Ok(BudgetLimit::EnvVar(lit.value()))
            }
        } else {
            let lit: LitInt = input.parse()?;
            Ok(BudgetLimit::Int(lit.base10_parse()?))
        }
    }
}

#[derive(Default)]
struct BudgetSpec {
    cpu: Option<BudgetLimit>,
    mem: Option<BudgetLimit>,
    env_ident: Option<Ident>,
}

impl Parse for BudgetSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut spec = BudgetSpec::default();

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

    let metric_label = match &metric {
        BudgetMetric::CpuInstructionCost => "budget_cpu_lt",
        BudgetMetric::MemoryBytesCost => "budget_mem_lt",
    };

    let limit_expr = match limit {
        BudgetLimit::Int(n) => quote! { #n },
        BudgetLimit::EnvVar(var) => quote! {
            match budget_env_resolve(#var) {
                Some(s) => s.parse::<u64>().unwrap_or_else(|_| {
                    panic!(
                        "{}: env var {}={:?} is not a valid u64",
                        #metric_label,
                        #var,
                        s
                    )
                }),
                None => u64::MAX,
            }
        },
    };

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        if spec.cpu.is_none() && spec.mem.is_none() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "must provide at least one of `cpu` or `mem` limits",
            ));
        }

        Ok(spec)
    }
}

fn generate_budget_assert(spec: BudgetSpec, item: TokenStream) -> TokenStream {
    let mut input_fn = match syn::parse2::<ItemFn>(item.into()) {
        Ok(f) => f,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    let stmts = &input_fn.block.stmts;

    let env_ident = spec
        .env_ident
        .unwrap_or_else(|| proc_macro2::Ident::new("env", proc_macro2::Span::call_site()));

    let mut asserts = Vec::new();

    if let Some(limit) = spec.cpu {
        let limit_expr = match limit {
            BudgetLimit::Int(n) => quote! { #n },
            BudgetLimit::EnvVar(var) => quote! {
                budget_env_resolve(#var)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(u64::MAX)
            },
        };
        let cost_ident = proc_macro2::Ident::new("cpu_cost", proc_macro2::Span::call_site());
        let cost_expr = quote! { budget.cpu_instruction_cost() };
        let assert_msg = "CPU instruction cost {} exceeded limit {} - local estimate, real network cost may differ significantly in either direction";

        asserts.push(quote! {
            let #cost_ident = #cost_expr;
            let limit_u64: u64 = #limit_expr;
            assert!(
                #cost_ident < limit_u64,
                #assert_msg,
                #cost_ident,
                limit_u64
            );
        });
    }

    if let Some(limit) = spec.mem {
        let limit_expr = match limit {
            BudgetLimit::Int(n) => quote! { #n },
            BudgetLimit::EnvVar(var) => quote! {
                budget_env_resolve(#var)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(u64::MAX)
            },
        };
        let cost_ident = proc_macro2::Ident::new("mem_cost", proc_macro2::Span::call_site());
        let cost_expr = quote! { budget.memory_bytes_cost() };
        let assert_msg = "Memory bytes cost {} exceeded limit {} - local estimate, real network cost may differ significantly in either direction";

        asserts.push(quote! {
            let #cost_ident = #cost_expr;
            let limit_u64: u64 = #limit_expr;
            assert!(
                #cost_ident < limit_u64,
                #assert_msg,
                #cost_ident,
                limit_u64
            );
        });
    }

    let new_block = quote! {
        {
            #[allow(unused_variables)]
            let budget_env_resolve = |var: &str| -> Option<String> {
                std::env::var(var).ok()
            };

            #(#stmts)*

            let budget = #env_ident.cost_estimate().budget();
            #(#asserts)*
        }
    };

    *input_fn.block = syn::parse2(new_block).unwrap();

    TokenStream::from(quote! {
        #input_fn
    })
}

/// Asserts that the CPU instructions used by `env` are less than N.
/// Must be placed on a test function that has a local `env` variable.
///
/// This checks a *local* estimate. Real network cost can differ from it
/// significantly in either direction depending on the build profile — see
/// `docs/src/mechanics.md` for measurements. Use `cargo budget-report` for
/// network ground truth.
///
/// When using `env = "VAR"`, an unset environment variable means "no limit"
/// (the assertion will always pass). The test will panic if the variable is
/// set but its value cannot be parsed as a `u64`.
#[proc_macro_attribute]
pub fn budget_cpu_lt(attr: TokenStream, item: TokenStream) -> TokenStream {
    let limit = match syn::parse2::<BudgetLimit>(attr.into()) {
        Ok(l) => l,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };
    let spec = BudgetSpec {
        cpu: Some(limit),
        mem: None,
        env_ident: None,
    };
    generate_budget_assert(spec, item)
}

/// Asserts that the memory bytes used by `env` are less than N.
/// Must be placed on a test function that has a local `env` variable.
///
/// This checks a *local* estimate. Real network cost can differ from it
/// significantly in either direction depending on the build profile — see
/// `docs/src/mechanics.md` for measurements. Use `cargo budget-report` for
/// network ground truth.
///
/// When using `env = "VAR"`, an unset environment variable means "no limit"
/// (the assertion will always pass). The test will panic if the variable is
/// set but its value cannot be parsed as a `u64`.
#[proc_macro_attribute]
pub fn budget_mem_lt(attr: TokenStream, item: TokenStream) -> TokenStream {
    let limit = match syn::parse2::<BudgetLimit>(attr.into()) {
        Ok(l) => l,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };
    let spec = BudgetSpec {
        cpu: None,
        mem: Some(limit),
        env_ident: None,
    };
    generate_budget_assert(spec, item)
}

/// Asserts that the CPU instructions and/or memory bytes used by the environment are less than specified limits.
/// Must be placed on a test function. The environment variable name defaults to `env` unless `env_ident` is provided.
///
/// Examples:
/// `#[budget_lt(cpu = 800000, mem = 200000)]`
/// `#[budget_lt(cpu = env("MAX_CPU"))]`
/// `#[budget_lt(mem = 200000, env_ident = test_env)]`
#[proc_macro_attribute]
pub fn budget_lt(attr: TokenStream, item: TokenStream) -> TokenStream {
    let spec = match syn::parse2::<BudgetSpec>(attr.into()) {
        Ok(s) => s,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };
    generate_budget_assert(spec, item)
}
