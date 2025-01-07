import { anvil } from "viem/chains";
import { createWalletClient, webSocket } from "viem";
import { eip7702Actions } from "viem/experimental";
import { privateKeyToAccount } from "viem/accounts";

import { abi as originSettlerAbi } from "../contracts/out/OriginSettler.sol/OriginSettler.json";

async function main() {
  console.log("starting xchain client");


  const account = privateKeyToAccount("0x00");

  const walletClient = createWalletClient({
    chain: anvil,
    transport: webSocket(),
  }).extend(eip7702Actions());

  const auth = walletClient.signAuthorization({
    account,
    contractAddress: "0x00",

  });
  const authlist = [auth];

  const asset = {
    address: "0x00",
    amount: 100
  }

  const callByUser = {
    address: account.address,
    nonce: 0,
    asset,
    chainId: 31337,
    signature: "0x00",
    calls: [],
  };

  console.log({ originSettlerAbi })
}

main();
