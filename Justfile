# Assumes you ran `make sandbox` in `../nearcore`.
near_sandbox_bin := "../nearcore/target/debug/neard-sandbox"

init_sandbox:
    {{ near_sandbox_bin }} --home ${NEAR_RUNNER_WS_NEAR_HOME} init

run_sandbox:
    {{ near_sandbox_bin }} --home ${NEAR_RUNNER_WS_NEAR_HOME} run
    
