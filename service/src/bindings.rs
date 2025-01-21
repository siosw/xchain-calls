use alloy::{
    eips::eip7702::{Authorization, SignedAuthorization},
    primitives::{PrimitiveSignature, U256},
    sol,
};
use std::str::FromStr;

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

impl TryInto<OriginSettler::Authorization> for SignedAuthorization {
    type Error = eyre::Error;
    fn try_into(self) -> Result<OriginSettler::Authorization, Self::Error> {
        Ok(OriginSettler::Authorization {
            chainId: self.chain_id,
            codeAddress: self.address,
            nonce: U256::from(self.nonce),
            signature: self.signature()?.as_bytes().into(),
        })
    }
}

// TODO: Vec<SignedAuthorization> into AuthData struct?

impl TryInto<SignedAuthorization> for &OriginSettler::Authorization {
    type Error = eyre::Error;
    fn try_into(self) -> Result<SignedAuthorization, Self::Error> {
        let inner = Authorization {
            chain_id: self.chainId,
            address: self.codeAddress,
            nonce: self.nonce.try_into().unwrap(),
        };
        let sig = PrimitiveSignature::from_str(&self.signature.to_string())?;
        Ok(SignedAuthorization::new_unchecked(
            inner,
            sig.v().into(),
            sig.r(),
            sig.s(),
        ))
    }
}

impl TryInto<Vec<SignedAuthorization>> for OriginSettler::EIP7702AuthData {
    type Error = eyre::Error;
    fn try_into(self) -> Result<Vec<SignedAuthorization>, Self::Error> {
        self.authlist.iter().map(|auth| auth.try_into()).collect()
    }
}
