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

	uint256 immutable AMOUNT = 100 * 1e18;

	OriginSettler origin;
	address token;
	Vm.Wallet user;

	function setUp() public {
		origin = new OriginSettler();

		token = address(deployMockERC20("Test Token", "TT", 18));
		user = vm.createWallet("User");
		deal(token, user.addr, AMOUNT, true);
	}

	function test_OnchainOrder() public {
		Call[] memory calls = new Call[](1);
		calls[0] = Call(address(0xdead), "", 0);

		Asset memory asset = Asset(token, AMOUNT);
		vm.prank(user.addr);
		IERC20(token).approve(address(origin), AMOUNT);

		CallByUser memory callByUser = CallByUser(address(0xdead), 0, asset, 1, "", calls);

		Authorization[] memory authlist = new Authorization[](1);
		authlist[0] = Authorization(1, address(0xdead), 0, "");
		
		EIP7702AuthData memory authData = EIP7702AuthData(authlist);

		bytes memory orderData = abi.encode(callByUser, authData, asset);


		bytes32 orderId = keccak256(abi.encode(calls));

		origin.pendingOrders(orderId);


		OnchainCrossChainOrder memory order = OnchainCrossChainOrder(
			type(uint32).max,
			origin.ORDER_DATA_TYPE_HASH(),
			orderData
		);

		vm.assertEq(IERC20(token).balanceOf(user.addr), AMOUNT);

		// TODO: assert correct events are emitted
		vm.prank(user.addr);
		origin.open(order);

		vm.assertEq(IERC20(token).balanceOf(user.addr), 0);
		vm.assertEq(IERC20(token).balanceOf(address(origin)), AMOUNT);

	}
}

