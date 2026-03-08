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

pub(super) fn generate_pyproject_toml(dep_count: usize) -> String {
    let mut content = String::from(
        "[project]\nname = \"test-project\"\nversion = \"0.1.0\"\ndescription = \"A test project\"\nrequires-python = \">=3.10\"\ndependencies = [\n",
    );

    for i in 0..dep_count {
        append_format(
            &mut content,
            format_args!(
                "    \"package{i}=={}.{}.{}\",\n",
                i / 100,
                (i / 10) % 10,
                i % 10
            ),
        );
    }

    content.push_str("]\n\n[project.optional-dependencies]\ndev = [\n");
    for i in 0..(dep_count / 3) {
        append_format(
            &mut content,
            format_args!("    \"dev_package{i}>=1.0.0\",\n"),
        );
    }
    content.push_str("]\n");

    content
}

pub(super) fn generate_pyproject_toml_with_extras(dep_count: usize) -> String {
    let mut content =
        String::from("[project]\nname = \"test-project\"\nversion = \"0.1.0\"\ndependencies = [\n");

    for i in 0..dep_count {
        let extra = if i % 5 == 0 {
            "ssl"
        } else if i % 5 == 1 {
            "cli"
        } else if i % 5 == 2 {
            "dev"
        } else {
            "full"
        };
        append_format(
            &mut content,
            format_args!(
                "    \"package{i}[{extra}]=={}.{}.{}\",\n",
                i / 100,
                (i / 10) % 10,
                i % 10
            ),
        );
    }

    content.push_str("]\n");
    content
}
