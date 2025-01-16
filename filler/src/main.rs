use std::u32;

use bindings::{DestinationSettler, MockERC20, OriginSettler, XAccount};
use eyre::OptionExt;

use alloy::{
    eips::eip7702::Authorization,
    network::{Ethereum, EthereumWallet, Network, TransactionBuilder7702},
    node_bindings::Anvil,
    primitives::{Address, Bytes, FixedBytes, U256},
    providers::{Provider, ProviderBuilder},
    signers::{local::PrivateKeySigner, Signer},
    sol_types::{SolEvent, SolValue},
    transports::Transport,
};
use OriginSettler::{EIP7702AuthData, ResolvedCrossChainOrder};

mod bindings;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // TODO:
    // - [x] submit order
    // - [x] listen for submitted orders
    // - [ ] execute call against settler (with auth data delegation)
    // - [ ] refactor into separate services
    //

    let anvil = Anvil::new().arg("--hardfork").arg("prague").try_spawn()?;
    let alice: PrivateKeySigner = anvil.keys()[0].clone().into();
    let bob: PrivateKeySigner = anvil.keys()[1].clone().into();

    // Create a provider with the wallet for only Bob (not Alice).
    let rpc_url = anvil.endpoint_url();
    let wallet = EthereumWallet::from(bob.clone());
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url);

    let token = MockERC20::deploy(&provider).await?;
    println!("deployed ERC20 on address: {}", token.address());

    let x_account = XAccount::deploy(&provider).await?;
    println!("deployed XAccount on address: {}", x_account.address());

    let origin = OriginSettler::deploy(&provider).await?;
    println!("deployed OriginSettler on address: {}", origin.address());

    let destination = DestinationSettler::deploy(&provider).await?;
    println!(
        "deployed DestintionSettler on address: {}",
        destination.address()
    );

    let tx_hash = submit_order(
        &provider,
        alice,
        *origin.address(),
        *token.address(),
        *x_account.address(),
    )
    .await?;

    fill_order(tx_hash, origin.address(), &provider).await?;

    Ok(())
}

async fn submit_order<P, T, N>(
    provider: P,
    signer: PrivateKeySigner,
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

    let auth = Authorization {
        address: x_account,
        nonce: provider.get_transaction_count(signer.address()).await?,
        chain_id: U256::from(chain_id),
    };
    let signature = signer.sign_hash(&auth.signature_hash()).await?;
    let auth = auth.into_signed(signature);
    let auth_data = OriginSettler::EIP7702AuthData {
        authlist: vec![auth.try_into()?],
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

async fn fill_order<P, T>(
    tx_hash: FixedBytes<32>,
    destination: &Address,
    provider: &P,
) -> eyre::Result<()>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    let destination = DestinationSettler::new(*destination, &provider);
    let tx = provider
        .get_transaction_receipt(tx_hash)
        .await?
        .ok_or_eyre("tx not found")?;

    // TODO: get order and authlist from tx
    let logs = tx.inner.logs();
    println!("total number of events: {:?}", logs.len());

    let mut id: Option<FixedBytes<32>> = None;
    let mut order: Option<ResolvedCrossChainOrder> = None;
    let mut delegation: Option<EIP7702AuthData> = None;

    for log in logs {
        match log.topic0() {
            Some(&OriginSettler::Open::SIGNATURE_HASH) => {
                let OriginSettler::Open {
                    orderId,
                    resolvedOrder,
                } = log.log_decode()?.inner.data;

                println!("order id: {}", orderId);
                id = Some(orderId);
                order = Some(resolvedOrder);
            }
            Some(&OriginSettler::Requested7702Delegation::SIGNATURE_HASH) => {
                let OriginSettler::Requested7702Delegation { authData } =
                    log.log_decode()?.inner.data;
                delegation = Some(authData);
            }
            Some(_) => {}
            None => {}
        };
        println!("\tevent from addr: {}", log.inner.address);
    }

    if let Some(order) = order {
        if let Some(id) = id {
            if let Some(delegation) = delegation {
                // TODO: send fill tx
                let tx = destination
                    .fill(id, order.abi_encode_params().into(), "".into())
                    .into_transaction_request()
                    .with_authorization_list(delegation.try_into()?);

                println!("{:?}", tx);
            }
        }
    }

    Ok(())
}
