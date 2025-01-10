// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import {Test, Vm, console} from "forge-std/Test.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

import {XAccount} from "../src/XAccount.sol";
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
	// uint256 immutable AMOUNT = 100 * 1e18;
	uint256 immutable AMOUNT = 0;

	XAccount account;
	OriginSettler origin;
	address token;
	Vm.Wallet user;

	function setUp() public {
		account = new XAccount();
		origin = new OriginSettler();

		token = address(deployMockERC20("Test Token", "TT", 18));
		user = vm.createWallet("User");
		deal(token, user.addr, AMOUNT, true);
	}

	function test_OnchainOrder() public {
		Call[] memory calls = new Call[](1);
		calls[0] = Call(address(0), "", 0);

		Asset memory asset = Asset(token, AMOUNT);
		vm.prank(user.addr);
		IERC20(token).approve(address(origin), AMOUNT);

		CallByUser memory callByUser = CallByUser(address(0), 0, asset, 1, "", calls);

		Authorization[] memory authlist = new Authorization[](1);
		authlist[0] = Authorization(1, address(account), 0, "");
		
		EIP7702AuthData memory authData = EIP7702AuthData(authlist);

		bytes memory userEncoded = abi.encode(callByUser);
		console.log("callByUser");
		console.log(userEncoded.length);
		console.logBytes(userEncoded);

		bytes memory authEncoded = abi.encode(authData);
		console.log("authData");
		console.log(authEncoded.length);
		console.logBytes(authEncoded);

		bytes memory assetEncoded = abi.encode(asset);
		console.log("assetData");
		console.log(assetEncoded.length);
		console.logBytes(assetEncoded);

		bytes memory orderData = abi.encode(callByUser, authData, asset);
		console.log("orderData");
		console.log(orderData.length);
		console.logBytes(orderData);

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

	function createOrder() public {
		uint256 chainId = 31337;
		address codeAddress = address(account);
		uint256 nonce = 0;

		bytes32 digest = keccak256(abi.encode(chainId, codeAddress, nonce));

		vm.sign(user, digest);

		Authorization[] memory authlist = new Authorization[](1);

		authlist[0] = Authorization(1, address(0xdead), 0, "");
		
		EIP7702AuthData memory authData = EIP7702AuthData(authlist);
	}
}

