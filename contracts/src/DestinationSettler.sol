// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {SignatureChecker} from "@openzeppelin/contracts/utils/cryptography/SignatureChecker.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {CallByUser, Call} from "./Structs.sol";
import {ResolvedCrossChainOrderLib} from "./ResolvedCrossChainOrderLib.sol";
import {XAccount} from "./XAccount.sol";

/**
 * @notice Destination chain entrypoint contract for fillers relaying cross chain message containing delegated
 * calldata.
 * @dev This is a simple escrow contract that is encouraged to be modified by different xchain settlement systems
 * that might want to add features such as exclusive filling, deadlines, fee-collection, etc.
 * @dev This could be replaced by the Across SpokePool, for example, which gives fillers many features with which
 * to protect themselves from malicious users and moreover allows them to provide transparent pricing to users.
 * However, this contract could be bypassed almost completely by lightweight settlement systems that could essentially
 * combine its logic with the XAccount contract to avoid the extra transferFrom and approve steps required in a more
 * complex escrow system.
 */
contract DestinationSettler is ReentrancyGuard {
    using SafeERC20 for IERC20;

    /// @notice Store unique orders to prevent duplicate fills for the same order.
    mapping(bytes32 => bool) public fillStatuses;

    error InvalidOrderId();
    error DuplicateFill();

    // Called by filler, who sees ERC7683 intent emitted on origin chain
    // containing the callsByUser data to be executed following a 7702 delegation.
    // @dev We don't use the last parameter `fillerData` in this function.
    function fill(bytes32 orderId, bytes calldata originData, bytes calldata) external nonReentrant {
        (CallByUser memory callsByUser) = abi.decode(originData, (CallByUser));
        if (ResolvedCrossChainOrderLib.getOrderId(callsByUser) != orderId) revert InvalidOrderId();

        // Protect against duplicate fills.
        if (fillStatuses[orderId]) revert DuplicateFill();
        fillStatuses[orderId] = true;

        // TODO: Protect fillers from collisions with other fillers. Requires letting user set an exclusive relayer.

        // Pull funds into this settlement contract and perform any steps necessary to ensure that filler
        // receives a refund of their assets.
        _fundAndApproveXAccount(callsByUser);

        // The following call will only succeed if the user has set a 7702 authorization to set its code
        // equal to the XAccount contract. The filler should have seen any auth data emitted in an OriginSettler
        // event on the sending chain.
        XAccount(payable(callsByUser.user)).xExecute(orderId, callsByUser);

        // Perform any final steps required to prove that filler has successfully filled the ERC7683 intent.
        // For example, we could emit an event containing a unique hash of the fill that could be proved
        // on the origin chain via a receipt proof + RIP7755.
        // e.g. emit Executed(orderId)
    }

    // Pull funds into this settlement contract as escrow and use to execute user's calldata. Escrowed
    // funds will be paid back to filler after this contract successfully verifies the settled intent.
    // This step could be skipped by lightweight escrow systems that don't need to perform additional
    // validation on the filler's actions.
    function _fundAndApproveXAccount(CallByUser memory call) internal {
        IERC20(call.asset.token).safeTransferFrom(msg.sender, address(this), call.asset.amount);
        IERC20(call.asset.token).forceApprove(call.user, call.asset.amount);
    }
}
