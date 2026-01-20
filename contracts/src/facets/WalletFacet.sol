// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "../libraries/LibDiamond.sol";

contract WalletFacet {
    event CallExecuted(address indexed target, uint256 value, bytes data);

    function executeCall(address _target, uint256 _value, bytes calldata _data) external payable returns (bytes memory result) {
        LibDiamond.enforceIsContractOwner();
        
        // Execute the call
        bool success;
        (success, result) = _target.call{value: _value}(_data);
        
        require(success, "WalletFacet: Call failed");
        
        emit CallExecuted(_target, _value, _data);
    }
    
    // Allow receiving ETH
    receive() external payable {}
}
