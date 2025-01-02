// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import {Script} from "forge-std/Script.sol";
import {XAccount} from "../src/XAccount.sol";
import {OriginSettler} from "../src/OriginSettler.sol";
import {DestinationSettler} from "../src/DestinationSettler.sol";

contract Deploy is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

	new XAccount();
	new OriginSettler();
	new DestinationSettler();

        vm.stopBroadcast();
    }
}
