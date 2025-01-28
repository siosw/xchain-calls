use clap::{Parser, Subcommand};
// use filler::Filler;

mod bindings;
mod calls;
mod chain;
mod filler;

#[derive(Subcommand, Debug)]
enum Commands {
    Fill,
    Generate,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long)]
    private_key: String,

    config: String,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::parse();

    // context create from args
    // context is input to filler / generator
    // context can also be created from anvil for internal e2e

    match &args.command {
        Commands::Fill => {
            // let filler = Filler::new();
        }
        Commands::Generate => {}
    };

    // TODO:
    // - [x] long running filler service
    // - [x] move current e2e flow into test
    // - [ ] cli opts to: generate / fill orders / set up system
    // - [ ] add logging
    // - [ ] submit issue for erc1271 & incorrect pending check
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::u32;

    use crate::bindings::{DestinationSettler, MockERC20, OriginSettler, XAccount};
    use crate::calls::{Asset, Call, SignedCrossChainCalls};
    use eyre::OptionExt;

    use crate::filler::{Filler, Order};
    use alloy::{
        eips::eip7702::Authorization,
        network::{EthereumWallet, Network},
        node_bindings::Anvil,
        primitives::{Address, Bytes, FixedBytes, U256},
        providers::{Provider, ProviderBuilder, WsConnect},
        signers::{local::PrivateKeySigner, Signer},
        sol_types::SolValue,
        transports::Transport,
    };

    #[tokio::test]
    async fn test_end_to_end() -> eyre::Result<()> {
        let anvil = Anvil::new().arg("--hardfork").arg("prague").try_spawn()?;
        let alice: PrivateKeySigner = anvil.keys()[0].clone().into();
        let bob: PrivateKeySigner = anvil.keys()[1].clone().into();

        let ws = WsConnect::new(anvil.ws_endpoint());
        let wallet = EthereumWallet::from(bob.clone());
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_ws(ws)
            .await?;

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

        let tx_hash = submit_test_order(
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

        let filler = Filler::new(
            &provider,
            &provider,
            origin.address(),
            destination.address(),
        );
        let tx = filler.fill(order).await?;
        println!("fill tx: {:?}", tx);

        Ok(())
    }

    async fn submit_test_order<P, T, N>(
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
}
