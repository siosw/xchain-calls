use std::u32;

use futures_util::StreamExt;

use alloy::{
    network::Network,
    primitives::{Address, Bytes, U256},
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
    // TODO: issue with anvil / typed hardforks

    // TODO:
    // - [x] submit order
    // - [ ] listen for submitted orders
    // - [ ] execute call against settler
    // - [ ] refactor into separate services

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_anvil_with_wallet_and_config(|anvil| {
            anvil.arg("--hardfork").arg("prague").arg("-vvvv")
        });

    let token = MockERC20::deploy(&provider).await?;
    println!("deployed ERC20 on address: {}", token.address());

    let x_account = XAccount::deploy(&provider).await?;
    println!("deployed XAccount on address: {}", x_account.address());

    let origin = OriginSettler::deploy(&provider).await?;
    println!("deployed OriginSettler on address: {}", origin.address());

    let open_filter = origin.Open_filter().watch().await?;
    let open_listener = open_filter.into_stream().take(1).for_each(|log| async {
        match log {
            Ok((_event, log)) => {
                println!("Received Open: {log:?}");
            }
            Err(e) => {
                println!("Error: {e:?}");
            }
        }
    });

    submit_order(
        &provider,
        *origin.address(),
        *token.address(),
        *x_account.address(),
    )
    .await?;
    open_listener.await;

    Ok(())
}

async fn submit_order<P, T, N>(
    provider: P,
    origin: Address,
    token: Address,
    x_account: Address,
) -> eyre::Result<()>
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
    Ok(())
}
