// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import {Test, Vm} from "forge-std/Test.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

import {OriginSettler} from "../src/OriginSettler.sol";
import {
    OnchainCrossChainOrder,
    GaslessCrossChainOrder,
    ResolvedCrossChainOrder,
    IOriginSettler,
    Output,
    FillInstruction
} from "../src/ERC7683.sol";
import {
	Call, 
	CallByUser,
	EIP7702AuthData,
	Authorization,
	Asset
} from "../src/Structs.sol";

contract OriginSettlerTest is Test {

	OriginSettler origin;
	address token;
	Vm.Wallet user;

	function setUp() public {
		origin = new OriginSettler();

		token = address(deployMockERC20("Test Token", "TT", 18));
		user = vm.createWallet("User");
		deal(token, user.addr, 100 * 1e18, true);
	}

	function test_OnchainOrder() public {
		Call[] memory calls = new Call[](1);
		calls[0] = Call(address(0), "", 0);

		Asset memory asset = Asset(token, 10 * 1e18);
		vm.prank(user.addr);
		IERC20(token).approve(address(origin), 10 * 1e18);

		CallByUser memory callByUser = CallByUser(address(0), 0, asset, 1, "", calls);

		Authorization[] memory authlist = new Authorization[](1);
		authlist[0] = Authorization(1, address(0), 0, "");
		
		EIP7702AuthData memory authData = EIP7702AuthData(authlist);

		bytes memory orderData = abi.encode(callByUser, authData, asset);


		bytes32 orderId = keccak256(abi.encode(calls));

		origin.pendingOrders(orderId);


		OnchainCrossChainOrder memory order = OnchainCrossChainOrder(
			type(uint32).max,
			origin.ORDER_DATA_TYPE_HASH(),
			orderData
		);

		vm.prank(user.addr);
		origin.open(order);

		// TODO: assert funds are held by OriginSettler
		// correct events are emitted
	}
}

