use std::fmt::Write as FmtWrite;

pub(super) const BENCH_SLACK_ENV: &str = "OMNI_WENDAO_BENCH_SLACK_FACTOR";
const DEFAULT_BENCH_SLACK_FACTOR: f64 = 2.0;

pub(super) fn benchmark_slack_factor() -> f64 {
    std::env::var(BENCH_SLACK_ENV)
        .ok()
        .and_then(|raw| raw.parse::<f64>().ok())
        .filter(|factor| factor.is_finite() && *factor >= 1.0)
        .unwrap_or(DEFAULT_BENCH_SLACK_FACTOR)
}

pub(super) fn benchmark_runtime_multiplier() -> f64 {
    if std::env::var_os("NEXTEST_RUN_ID").is_some() {
        6.0
    } else {
        1.0
    }
}

pub(super) fn benchmark_budget(
    local: std::time::Duration,
    ci: std::time::Duration,
) -> std::time::Duration {
    let baseline = if std::env::var_os("CI").is_some() {
        ci
    } else {
        local
    };
    baseline.mul_f64(benchmark_slack_factor() * benchmark_runtime_multiplier())
}

fn append_format(content: &mut String, args: std::fmt::Arguments<'_>) {
    if content.write_fmt(args).is_err() {
        unreachable!("formatting into String should not fail");
    }
}

pub(super) fn generate_rust_test_file(line_count: usize) -> String {
    let mut content = String::with_capacity(line_count * 50);

    for i in 0..(line_count / 50) {
        append_format(
            &mut content,
            format_args!(
                "pub struct Struct{i} {{\n    field_{i}: String,\n    field_{i}: i32,\n}}\n"
            ),
        );
    }

    for i in 0..(line_count / 100) {
        append_format(
            &mut content,
            format_args!(
                "pub enum Enum{i} {{\n    VariantA,\n    VariantB(i32),\n    VariantC {{ x: i32, y: i32 }},\n}}\n"
            ),
        );
    }

    for i in 0..(line_count / 30) {
        append_format(
            &mut content,
            format_args!(
                "pub fn function_{i}(arg1: &str, arg2: i32) -> Result<(), Box<dyn std::error::Error>> {{\n    let _result = process_data(arg1, arg2);\n    Ok(())\n}}\n"
            ),
        );
    }

    for i in 0..(line_count / 80) {
        append_format(
            &mut content,
            format_args!(
                "pub trait Trait{i} {{\n    fn method_a(&self) -> i32;\n    fn method_b(&self, x: i32) -> bool;\n}}\n"
            ),
        );
    }

    content
}

pub(super) fn generate_python_test_file(line_count: usize) -> String {
    let mut content = String::with_capacity(line_count * 40);

    for i in 0..(line_count / 50) {
        append_format(
            &mut content,
            format_args!(
                "class Class{i}:\n    def __init__(self, param_a: str, param_b: int):\n        self.param_a = param_a\n        self.param_b = param_b\n\n    def method_a(self) -> str:\n        return self.param_a.upper()\n\n    def method_b(self, value: int) -> bool:\n        return value > 0\n\n    async def async_method(self) -> dict:\n        return {{\"status\": \"ok\"}}\n"
            ),
        );
    }

    for i in 0..(line_count / 20) {
        append_format(
            &mut content,
            format_args!(
                "def function_{i}(arg1: str, arg2: int) -> bool:\n    \"\"\"Process data and return result.\"\"\"\n    result = process(arg1, arg2)\n    return result\n\nasync def async_function_{i}(data: dict) -> list:\n    \"\"\"Async data processing.\"\"\"\n    results = []\n    return results\n"
            ),
        );
    }

    content
}
