use std::env;

use near_sdk::json_types::U128;
use near_sdk::NearToken;
use near_workspaces::network::{Sandbox, ValidatorKey};
use near_workspaces::result::ExecutionFinalResult;
use near_workspaces::{Account, AccountId, Contract, Worker};
use serde_json::json;

// Consider making these parameters of a CLI.
pub const FT_WASM_FILEPATH: &str =
    "./contracts/ft/target/wasm32-unknown-unknown/release/fungible_token.wasm";
const RPC_ADDRESS: &str = "http://localhost:3030";
const NEAR_HOME_ENV_VAR: &str = "NEAR_RUNNER_WS_NEAR_HOME";

pub async fn connect_to_sandbox() -> anyhow::Result<Worker<Sandbox>> {
    let home_dir =
        env::var(NEAR_HOME_ENV_VAR).expect(&format!("{NEAR_HOME_ENV_VAR} should be set"));
    // TODO `home_dir` method doesn't exist. Send workspaces PR.
    let worker = near_workspaces::sandbox()
        .rpc_addr(RPC_ADDRESS)
        .validator_key(ValidatorKey::HomeDir(home_dir.as_str().into()))
        .await?;
    Ok(worker)
}

fn assert_success(result: ExecutionFinalResult) {
    let res = result.into_result();
    match res {
        Ok(_) => {}
        Err(err) => panic!(
            "Transaction was expected to succeed but failed with:\n{:#?}",
            err
        ),
    }
}

async fn init_ft_contract(
    ft_contract: &Contract,
    owner_id: &AccountId,
) -> anyhow::Result<ExecutionFinalResult> {
    let result = ft_contract
        .call("new_default_meta")
        .args_json(json!({
            "owner_id": owner_id,
            "total_supply": 10e8.to_string(),
        }))
        .transact()
        .await?;
    Ok(result)
}

async fn ft_transfer(
    ft_contract: &AccountId,
    sender: &Account,
    receiver: &AccountId,
    amount: &str,
) -> anyhow::Result<ExecutionFinalResult> {
    let result = sender
        .call(ft_contract, "ft_transfer")
        .args_json(json!({
            "receiver_id": receiver,
            "amount": amount,
        }))
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await?;
    Ok(result)
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
    use near_contract_standards::fungible_token::FungibleToken;
    use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
    use near_sdk::collections::LazyOption;
    use near_sdk::BorshStorageKey;
    use near_workspaces::result::ExecutionFinalResult;
    use near_workspaces::AccountId;

    #[tokio::test]
    async fn test_observing_gas_limit() -> anyhow::Result<()> {
        let worker = connect_to_sandbox().await?;
        let wasm = std::fs::read(FT_WASM_FILEPATH)?;
        let _contract = worker.dev_deploy(&wasm).await?;

        let ch = worker.view_chunk().await?.header;
        println!(
            "gas_limit of {} at chunk included at height {}",
            ch.gas_limit, ch.height_included
        );

        // Deploy contract another time to let chain advance.
        let _contract = worker.dev_deploy(&wasm).await?;

        let ch = worker.view_chunk().await?.header;
        println!(
            "gas_limit of {} at chunk included at height {}",
            ch.gas_limit, ch.height_included
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_passive_ft_user_generation() -> anyhow::Result<()> {
        let worker = connect_to_sandbox().await?;
        let wasm = std::fs::read(FT_WASM_FILEPATH)?;
        let contract = worker.dev_deploy(&wasm).await?;

        // Contract initialization.
        let token_owner = worker.dev_create_account().await?;
        let result = init_ft_contract(&contract, token_owner.id()).await?;
        assert_success(result);

        // Transfering to an unregistered account fails.
        let receiver = worker.dev_create_account().await?;
        let result = ft_transfer(contract.id(), &token_owner, receiver.id(), "42").await?;
        let expected_err = format!("The account {} is not registered", receiver.id());
        assert_failure_with(result, &expected_err);

        // Patching state to register `receiver` with the ft contract
        //
        // Ultimately `receiver` is registered by the contract calling
        // token.accounts.insert(receiver.id(), &0);
        //
        // Intuitively this requires getting the contracts state, calling that `insert` and then
        // writing the patched state back to the contract's storage on chain.
        // However, that's not how near-sdk's `LookupMap` works. Inserting a value into a
        // `LookupMap` instead adds a new entry to the contract's top level storage. Let's do that.

        let key_to_insert = [
            &[0, 33, 0, 0, 0], // the `LookupMap` storage prefix assigned by the contract
            receiver.id().as_bytes(),
        ]
        .concat();
        worker
            .patch_state(contract.id(), &key_to_insert, &borsh::to_vec(&0u128)?)
            .await?;

        // Verify now the user is registered and the transfer succeeds.
        let result = ft_transfer(contract.id(), &token_owner, receiver.id(), "42").await?;
        assert_success(result);

        Ok(())
    }

    /// Asserts the execution of `res` failed and the error contains `must_contain`.
    fn assert_failure_with(result: ExecutionFinalResult, must_contain: &str) {
        let err = result
            .into_result()
            .expect_err("Transaction should have failed");
        let err = format!("{}", err);
        assert!(
            err.contains(must_contain),
            "The expected message\n'{}'\nis not contained in error\n'{}'",
            must_contain,
            err,
        );
    }

    async fn get_state(
        worker: &Worker<Sandbox>,
        contract_id: &AccountId,
    ) -> anyhow::Result<Contract> {
        let map = worker.view_state(contract_id).await?;
        for key in map.keys() {
            println!("key: {:?}", key);
        }
        let bytes = worker
            .view_state(contract_id)
            .await?
            .remove(b"STATE".as_slice())
            .expect("STATE should be present");
        let contract = Contract::try_from_slice(&bytes)?;
        Ok(contract)
    }

    // Mirrors `contracts/ft/src/lib.rs`
    // TODO import the struct
    #[derive(BorshSerialize, BorshDeserialize)]
    #[borsh(crate = "near_sdk::borsh")]
    pub struct Contract {
        token: FungibleToken,
        metadata: LazyOption<FungibleTokenMetadata>,
    }

    #[derive(BorshSerialize, BorshStorageKey)]
    #[borsh(crate = "near_sdk::borsh")]
    pub enum StorageKey {
        FungibleToken,
        Metadata,
    }
}
