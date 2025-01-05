// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import {Script, console} from "forge-std/Script.sol";
import {XAccount} from "../src/XAccount.sol";
import {OriginSettler} from "../src/OriginSettler.sol";
import {DestinationSettler} from "../src/DestinationSettler.sol";

contract Deploy is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

	XAccount account = new XAccount();
	OriginSettler origin = new OriginSettler();
	DestinationSettler destination = new DestinationSettler();

        vm.stopBroadcast();

	console.log("XAccount:", address(account));
	console.log("OriginSettler:", address(origin));
	console.log("DestinationSettler:", address(destination));
    }
}
