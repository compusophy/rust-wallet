// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IERC173 {
    /// @notice Get the owner of this contract
    /// @return owner_ The address of the owner.
    function owner() external view returns (address owner_);

    /// @notice Set the owner of this contract
    /// @param _newOwner The address of the new owner.
    function transferOwnership(address _newOwner) external;
}
