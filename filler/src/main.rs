use std::u32;

use eyre::OptionExt;

use alloy::{
    network::{Ethereum, Network},
    primitives::{Address, Bytes, FixedBytes, U256},
    providers::{Provider, ProviderBuilder},
    sol,
    sol_types::SolValue,
    transports::Transport,
};

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    MockERC20,
    "../contracts/out/MockERC20.sol/MockERC20.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    OriginSettler,
    "../contracts/out/OriginSettler.sol/OriginSettler.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    DestinationSettler,
    "../contracts/out/DestinationSettler.sol/DestinationSettler.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    XAccount,
    "../contracts/out/XAccount.sol/XAccount.json"
);

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // TODO:
    // - [x] submit order
    // - [x] listen for submitted orders
    // - [ ] execute call against settler (with auth data delegation)
    // - [ ] refactor into separate services

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_anvil_with_wallet_and_config(|anvil| anvil.arg("--hardfork").arg("prague"));

    let token = MockERC20::deploy(&provider).await?;
    println!("deployed ERC20 on address: {}", token.address());

    let x_account = XAccount::deploy(&provider).await?;
    println!("deployed XAccount on address: {}", x_account.address());

    let origin = OriginSettler::deploy(&provider).await?;
    println!("deployed OriginSettler on address: {}", origin.address());

    //let open_filter = origin.Open_filter().watch().await?;
    //let open_listener = open_filter.into_stream().take(1).for_each(|log| async {
    //    match log {
    //        Ok((_event, _log)) => {
    //            println!("received open event");
    //        }
    //        Err(e) => {
    //            println!("Error: {e:?}");
    //        }
    //    }
    //});
    //
    //let auth_filter = origin.Requested7702Delegation_filter().watch().await?;
    //let auth_listener = auth_filter.into_stream().take(1).for_each(|log| async {
    //    match log {
    //        Ok((_event, _log)) => {
    //            println!("received auth event");
    //        }
    //        Err(e) => {
    //            println!("Error: {e:?}");
    //        }
    //    }
    //});

    let tx_hash = submit_order(
        &provider,
        *origin.address(),
        *token.address(),
        *x_account.address(),
    )
    .await?;

    fill_order(tx_hash, &provider).await?;

    Ok(())
}

async fn submit_order<P, T, N>(
    provider: P,
    origin: Address,
    token: Address,
    x_account: Address,
) -> eyre::Result<FixedBytes<32>>
where
    P: Provider<T, N>,
    T: Transport + Clone,
    N: Network,
{
    let chain_id = provider.get_chain_id().await?;

    let call = OriginSettler::Call {
        target: Address::ZERO,
        callData: "".into(),
        value: U256::ZERO,
    };
    let asset = OriginSettler::Asset {
        token,
        amount: U256::ZERO,
    };
    let user_call = OriginSettler::CallByUser {
        user: Address::ZERO,
        asset: asset.clone(),
        chainId: chain_id,
        nonce: U256::ZERO,
        calls: vec![call],
        signature: "".into(),
    };
    let auth = OriginSettler::Authorization {
        chainId: chain_id.try_into()?,
        nonce: U256::ZERO,
        signature: "".into(),
        codeAddress: x_account,
    };
    let auth_data = OriginSettler::EIP7702AuthData {
        authlist: vec![auth],
    };

    let order_data: Bytes = (user_call, auth_data, asset).abi_encode_params().into();

    let origin = OriginSettler::new(origin, provider);
    let builder = origin.ORDER_DATA_TYPE_HASH();
    let type_hash = builder.call().await?._0;

    let order = OriginSettler::OnchainCrossChainOrder {
        orderDataType: type_hash,
        orderData: order_data,
        fillDeadline: u32::MAX,
    };

    let builder = origin.open(order);
    let tx_hash = builder.send().await?.watch().await?;
    println!("opened order with tx hash: {}", tx_hash);

    Ok(tx_hash)
}

async fn fill_order<P, T>(tx_hash: FixedBytes<32>, provider: &P) -> eyre::Result<()>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    let tx = provider
        .get_transaction_receipt(tx_hash)
        .await?
        .ok_or_eyre("tx not found")?;

    // TODO: get order and authlist from tx
    let logs = tx.inner.logs();
    println!("total number of events: {:?}", logs.len());
    for log in logs {
        println!("\tevent from addr: {}", log.inner.address);
    }

    // TODO: submit tx to destination chain

    Ok(())
}
