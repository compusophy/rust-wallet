// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./libraries/LibDiamond.sol";
import "./interfaces/IDiamondCut.sol";

interface IERC165 {
    function supportsInterface(bytes4 interfaceId) external view returns (bool);
}

interface IERC6551Account {
    receive() external payable;
    function token() external view returns (uint256 chainId, address tokenContract, uint256 tokenId);
    function state() external view returns (uint256);
    function isValidSigner(address signer, bytes calldata context) external view returns (bytes4 magicValue);
}

interface IERC721 {
    function ownerOf(uint256 tokenId) external view returns (address);
}

contract DiamondTBA is IERC6551Account, IDiamondCut {
    uint256 public constant state = 1; // Simple state for now

    receive() external payable {}

    // ERC-6551: Get Token details from footer (Assuming minimal proxy usage)
    function token() public view override returns (uint256 chainId, address tokenContract, uint256 tokenId) {
        bytes memory footer = new bytes(0x60);
        assembly {
            extcodecopy(address(), add(footer, 0x20), sub(codesize(), 0x60), 0x60)
        }
        return abi.decode(footer, (uint256, address, uint256));
    }

    // ERC-6551: Check signer (owner of NFT)
    function isValidSigner(address signer, bytes calldata) external view override returns (bytes4) {
        if (_isValidSigner(signer)) {
            return 0x523e3260; // IERC6551Account.isValidSigner.selector
        }
        return 0x00000000;
    }

    function _isValidSigner(address signer) internal view returns (bool) {
        (uint256 chainId, address tokenContract, uint256 tokenId) = token();
        if (chainId != block.chainid) return false;
        try IERC721(tokenContract).ownerOf(tokenId) returns (address owner) {
            return owner == signer;
        } catch {
            return false;
        }
    }

    // Helper for initialization (since we are a proxy)
    function initialize(IDiamondCut.FacetCut[] calldata _diamondCut, address _init, bytes calldata _calldata) external {
        LibDiamond.DiamondStorage storage ds = LibDiamond.diamondStorage();
        // Simple protection: Check if we have facets or if contractOwner is set.
        // But contractOwner logic is changing.
        // Instead, check if any facets exist.
        require(ds.facetAddresses.length == 0, "Already initialized");
        
        LibDiamond.diamondCut(_diamondCut, _init, _calldata);
    }

    // IDiamondCut implementation (Protected by ownership)
    function diamondCut(
        FacetCut[] calldata _diamondCut,
        address _init,
        bytes calldata _calldata
    ) external override {
        require(_isValidSigner(msg.sender), "Not Owner");
        LibDiamond.diamondCut(_diamondCut, _init, _calldata);
    }

    // Fallback to Facets
    fallback() external payable {
        LibDiamond.DiamondStorage storage ds = LibDiamond.diamondStorage();
        address facet = ds.selectorToFacetAndPosition[msg.sig].facetAddress;
        require(facet != address(0), "Diamond: Function does not exist");
        
        assembly {
            calldatacopy(0, 0, calldatasize())
            let result := delegatecall(gas(), facet, 0, calldatasize(), 0, 0)
            returndatacopy(0, 0, returndatasize())
            switch result
                case 0 { revert(0, returndatasize()) }
                default { return(0, returndatasize()) }
        }
    }
}
