// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import {Test} from "forge-std/Test.sol";

import {OriginSettler} from "../src/OriginSettler.sol";
import {
    OnchainCrossChainOrder,
    GaslessCrossChainOrder,
    ResolvedCrossChainOrder,
    IOriginSettler,
    Output,
    FillInstruction
} from "../src/ERC7683.sol";

contract OriginSettlerTest is Test {

	OriginSettler origin;

	function setUp() public {
		// deploy O
		origin = new OriginSettler();
	}

	function test_OnchainOrder() public {
		OnchainCrossChainOrder memory order = OnchainCrossChainOrder(
			type(uint32).max,
			0x0,
			bytes("some")
		);

		origin.open(order);
	}
}

