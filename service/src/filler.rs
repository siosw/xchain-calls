use std::marker::PhantomData;

use alloy::{
    eips::{eip7702::SignedAuthorization, BlockNumberOrTag},
    network::{Ethereum, TransactionBuilder7702},
    primitives::{Address, Bytes, FixedBytes, LogData},
    providers::Provider,
    rpc::types::{Filter, Log},
    sol_types::SolEvent,
    transports::Transport,
};
use eyre::OptionExt;
use futures_util::StreamExt;

use crate::bindings::{DestinationSettler, OriginSettler};

const OPEN_TOPIC: Option<&FixedBytes<32>> = Some(&OriginSettler::Open::SIGNATURE_HASH);
const DELEGATION_TOPIC: Option<&FixedBytes<32>> =
    Some(&OriginSettler::Requested7702Delegation::SIGNATURE_HASH);

#[derive(Debug, Clone)]
pub struct Order {
    id: FixedBytes<32>,
    data: Bytes,
    auth_list: Vec<SignedAuthorization>,
}

impl TryFrom<&[Log<LogData>]> for Order {
    type Error = eyre::Error;
    fn try_from(logs: &[Log<LogData>]) -> Result<Self, Self::Error> {
        let open_event = logs
            .iter()
            .find(|log| log.topic0() == OPEN_TOPIC)
            .ok_or_eyre("logs have no open event")?;

        let OriginSettler::Open {
            orderId,
            resolvedOrder,
        } = open_event.log_decode()?.inner.data;

        let id = orderId;
        let data = resolvedOrder
            .fillInstructions
            .first()
            .unwrap()
            .originData
            .clone();

        let delegation_event = logs.iter().find(|log| log.topic0() == DELEGATION_TOPIC);

        let auth_list = if let Some(log) = delegation_event {
            let OriginSettler::Requested7702Delegation { authData } = log.log_decode()?.inner.data;
            authData.try_into()?
        } else {
            Vec::new()
        };

        Ok(Self {
            id,
            data,
            auth_list,
        })
    }
}

pub struct Filler<'a, P, T> {
    orig_p: P,
    dest_p: P,
    origin: &'a Address,
    destination: &'a Address,
    _phantom: PhantomData<T>,
}

impl<'a, P, T> Filler<'a, P, T>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    pub fn new(orig_p: P, dest_p: P, origin: &'a Address, destination: &'a Address) -> Self {
        Self {
            orig_p,
            dest_p,
            origin,
            destination,
            _phantom: PhantomData,
        }
    }

    pub async fn run(&self) -> eyre::Result<()> {
        let filter = Filter::new()
            .address(*self.origin)
            .from_block(BlockNumberOrTag::Latest);
        let sub = self.orig_p.subscribe_logs(&filter).await?;
        let mut stream = sub.into_stream();

        while let Some(log) = stream.next().await {
            let Some(hash) = log.transaction_hash else {
                continue;
            };
            let Ok(Some(tx)) = self.orig_p.get_transaction_receipt(hash).await else {
                continue;
            };
            println!("order created by tx: {:?}", tx);
            let Ok(order) = Order::try_from(tx.inner.logs()) else {
                continue;
            };

            println!("filling order: {:?}", order.id);
            let _ = self.fill(order).await;
        }

        Ok(())
    }

    pub async fn fill(&self, order: Order) -> eyre::Result<FixedBytes<32>> {
        let destination = DestinationSettler::new(*self.destination, &self.dest_p);
        let tx = destination
            .fill(order.id, order.data, Bytes::new())
            .into_transaction_request()
            .with_authorization_list(order.auth_list);
        let tx = self.orig_p.send_transaction(tx).await?;

        Ok(*tx.tx_hash())
    }
}
