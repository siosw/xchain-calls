use std::u32;

use bindings::{DestinationSettler, MockERC20, OriginSettler, XAccount};
use calls::{Asset, Call, SignedCrossChainCalls};
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
use filler::{Filler, Order};
use OriginSettler::{EIP7702AuthData, ResolvedCrossChainOrder};

mod bindings;
mod calls;
mod filler;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // TODO:
    // - [ ] long running filler service
    // - [ ] submit issue for erc1271 & incorrect pending check

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

    let tx = provider
        .get_transaction_receipt(tx_hash)
        .await?
        .ok_or_eyre("tx not found")?;

    let logs = tx.inner.logs();
    let order = Order::try_from(logs)?;
    println!("recovered order: {:?}", order);

    let filler = Filler::new(&provider, destination.address());
    let tx = filler.fill(order).await?;
    println!("fill tx: {:?}", tx);

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

    let call = Call {
        target: Address::ZERO,
        data: "".into(),
        value: U256::ZERO,
    };
    let destination_asset = Asset {
        token,
        amount: U256::ZERO,
    };
    let signed_calls = SignedCrossChainCalls {
        calls: vec![call],
        asset: destination_asset,
        nonce: provider.get_transaction_count(signer.address()).await?,
        destination_chain: chain_id,
        signer: signer.clone(),
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

    let origin_asset = Asset {
        token,
        amount: U256::ZERO,
    };

    let data: (
        OriginSettler::CallByUser,
        OriginSettler::EIP7702AuthData,
        OriginSettler::Asset,
    ) = (signed_calls.try_into()?, auth_data, origin_asset.into());
    let data: Bytes = data.abi_encode_params().into();

    let origin = OriginSettler::new(origin, provider);
    let builder = origin.ORDER_DATA_TYPE_HASH();
    let type_hash = builder.call().await?._0;

    let order = OriginSettler::OnchainCrossChainOrder {
        orderDataType: type_hash,
        orderData: data,
        fillDeadline: u32::MAX,
    };

    let builder = origin.open(order);
    let tx_hash = builder.send().await?.watch().await?;
    println!("opened order with tx hash: {}", tx_hash);

    Ok(tx_hash)
}
