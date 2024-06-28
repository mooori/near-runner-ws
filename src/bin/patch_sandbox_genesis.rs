use std::env;

// TODO find a way to import that constan from `src/lib.rs`
const NEAR_HOME_ENV_VAR: &str = "NEAR_RUNNER_WS_NEAR_HOME";

/// Needs to be run once after the local near sandbox was initialized to fix
/// https://github.com/near/near-workspaces-rs/issues/354
fn main() -> anyhow::Result<()> {
    let home_dir =
        env::var(NEAR_HOME_ENV_VAR).expect(&format!("{NEAR_HOME_ENV_VAR} should be set"));
    // TODO this requires near-workspaces-rs#360
    near_workspaces::network::set_sandbox_genesis(&home_dir)?;
    Ok(())
}
