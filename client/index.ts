import { anvil } from "viem/chains";
import { createWalletClient, webSocket } from "viem";
import { eip7702Actions } from "viem/experimental";

async function main() {
  console.log("starting xchain client");

  const walletClient = createWalletClient({
    chain: anvil,
    transport: webSocket(),
  }).extend(eip7702Actions());

}

main();
