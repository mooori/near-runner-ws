use futures::stream::FuturesUnordered;
use futures::StreamExt;
use near_runner_ws::{
    assert_success, create_ft_transfer_tx, register_ft_receiver, FT_WASM_FILEPATH,
};
use near_runner_ws::{connect_to_sandbox, ft_transfer, init_ft_contract};
use near_workspaces::{Account, AccountId, Contract, Worker};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let worker = connect_to_sandbox().await?;
    let wasm = std::fs::read(FT_WASM_FILEPATH)?;
    let ft_contract = worker.dev_deploy(&wasm).await?;

    let token_owner = worker.dev_create_account().await?;
    let result = init_ft_contract(&ft_contract, token_owner.id()).await?;
    assert_success(result);

    let receiver = worker.dev_create_account().await?;
    let result = register_ft_receiver(ft_contract.id(), &token_owner, receiver.id()).await?;
    assert_success(result);

    // Can be used to verify transfer works.
    // let result = ft_transfer(ft_contract.id(), &token_owner, receiver.id(), "42").await?;
    // assert_success(result);

    let timer = std::time::Instant::now();
    let tasks = FuturesUnordered::new();
    for i in 0..2000 {
        tasks.push(execute_ft_transfer(
            ft_contract.id(),
            &token_owner,
            receiver.id(),
            "42",
        ))
    }
    println!("creating futures took {}", timer.elapsed().as_secs());
    let timer = std::time::Instant::now();
    let _results: Vec<_> = tasks.collect().await;
    println!("executing futures took {}", timer.elapsed().as_secs());
    Ok(())
}

async fn execute_ft_transfer(
    ft_contract: &AccountId,
    sender: &Account,
    receiver: &AccountId,
    amount: &str,
) {
    // TODO check result
    let _result = create_ft_transfer_tx(ft_contract, sender, receiver, amount)
        .transact_async()
        .await
        .expect("tx should succeed");
    // assert_success(result);
}
