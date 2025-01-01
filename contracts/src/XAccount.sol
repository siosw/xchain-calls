// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SignatureChecker} from "@openzeppelin/contracts/utils/cryptography/SignatureChecker.sol";
import {CallByUser, Call} from "./Structs.sol";

/**
 * @notice Singleton contract used by all users who want to sign data on origin chain and delegate execution of
 * their calldata on this chain to this contract.
 */
contract XAccount is ReentrancyGuard {
    using SafeERC20 for IERC20;

    error CallReverted(uint256 index, Call[] calls);
    error InvalidCall(uint256 index, Call[] calls);
    error DuplicateExecution();
    error InvalidExecutionChainId();
    error InvalidUserSignature();

    /// @notice Store unique user ops to prevent duplicate executions.
    mapping(bytes32 => bool) public executionStatuses;

    /**
     * @notice Entrypoint function to be called by DestinationSettler contract on this chain. Should pull funds
     * to user's EOA and then execute calldata.
     * @dev Assume user has 7702-delegated code already to this contract.
     * @dev All calldata and 7702 authorization data is assumed to have been emitted on the origin chain in am ERC7683
     * intent creation event.
     */
    function xExecute(bytes32 orderId, CallByUser memory userCalls) external nonReentrant {
        if (executionStatuses[orderId]) revert DuplicateExecution();
        executionStatuses[orderId] = true;

        // Verify that the user signed the data blob.
        _verifyCalls(userCalls);
        // Verify that any included 7702 authorization data is as expected.
        _verify7702Delegation();
        _fundUser(userCalls);

        // TODO: Should we allow user to handle case where the calls fail and they want to specify
        // a fallback recipient? This might not be neccessary since the user will have pulled funds
        // into their account so worst case they'll still have access to those funds.
        _attemptCalls(userCalls.calls);
    }

    function _verifyCalls(CallByUser memory userCalls) internal view {
        if (userCalls.chainId != block.chainid) revert InvalidExecutionChainId();
        // @dev address(this) should be the userCall.user's EOA.
        // TODO: Make the blob to sign EIP712-compatible (i.e. instead of keccak256(abi.encode(...)) set
        // this to SigningLib.getTypedDataHash(...)
        if (
            !SignatureChecker.isValidSignatureNow(
                address(this), keccak256(abi.encode(userCalls.calls, userCalls.nonce)), userCalls.signature
            )
        ) revert InvalidUserSignature();
    }

    function _verify7702Delegation() internal {
        // TODO: We might not need this function at all, because if the authorization data requires that this contract
        // is set as the delegation code, then xExecute would fail if the auth data is not submitted by the filler.
        // However, it might still be useful to verify that the delegate is set correctly, like checking EXTCODEHASH.
    }

    function _attemptCalls(Call[] memory calls) internal {
        for (uint256 i = 0; i < calls.length; ++i) {
            Call memory call = calls[i];

            // If we are calling an EOA with calldata, assume target was incorrectly specified and revert.
            if (call.callData.length > 0 && call.target.code.length == 0) {
                revert InvalidCall(i, calls);
            }

            (bool success,) = call.target.call{value: call.value}(call.callData);
            if (!success) revert CallReverted(i, calls);
        }
    }

    function _fundUser(CallByUser memory call) internal {
        IERC20(call.asset.token).safeTransferFrom(msg.sender, call.user, call.asset.amount);
    }

    // Used if the caller is trying to unwrap the native token to this contract.
    receive() external payable {}
}
