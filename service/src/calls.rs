use crate::bindings::OriginSettler;
use alloy::{
    primitives::{keccak256, Address, Bytes, U256},
    signers::{local::PrivateKeySigner, SignerSync},
    sol_types::SolValue,
};

pub struct Call {
    pub target: Address,
    pub data: Bytes,
    pub value: U256,
}

pub struct Asset {
    pub token: Address,
    pub amount: U256,
}

pub struct SignedCrossChainCalls {
    pub calls: Vec<Call>,
    pub asset: Asset,
    pub nonce: u64,
    pub destination_chain: u64,
    pub signer: PrivateKeySigner,
}

impl SignedCrossChainCalls {
    // XAccount contract does not currently use EIP-712
    fn signature(&self) -> eyre::Result<Bytes> {
        let calls: Vec<OriginSettler::Call> = self.calls.iter().map(|c| c.into()).collect();
        let nonce = U256::from(self.nonce);
        let encoded: Bytes = (calls, nonce).abi_encode_params().into();
        let hash = keccak256(encoded);

        Ok(self.signer.sign_hash_sync(&hash)?.as_bytes().into())
    }
}

impl Into<OriginSettler::Asset> for Asset {
    fn into(self) -> OriginSettler::Asset {
        OriginSettler::Asset {
            token: self.token,
            amount: self.amount,
        }
    }
}

impl Into<OriginSettler::Call> for &Call {
    fn into(self) -> OriginSettler::Call {
        OriginSettler::Call {
            target: self.target,
            callData: self.data.clone(),
            value: self.value,
        }
    }
}

impl TryInto<OriginSettler::CallByUser> for SignedCrossChainCalls {
    type Error = eyre::Error;
    fn try_into(self) -> Result<OriginSettler::CallByUser, Self::Error> {
        let signature = self.signature()?;

        Ok(OriginSettler::CallByUser {
            user: self.signer.address(),
            nonce: U256::from(self.nonce),
            asset: self.asset.into(),
            chainId: self.destination_chain,
            signature,
            calls: self.calls.iter().map(|c| c.into()).collect(),
        })
    }
}
