use budget_macros::budget_lt;

#[budget_lt(cpu = 100, cpu = 200)]
fn test() {}

fn main() {}
