// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./Diamond.sol";
import "./interfaces/IDiamondCut.sol";
import "./interfaces/IDiamondLoupe.sol";
import "./interfaces/IERC173.sol";

contract WalletFactory {
    // Immutable addresses of the facets already deployed
    address public immutable diamondCutFacet;
    address public immutable diamondLoupeFacet;
    address public immutable ownershipFacet;
    address public immutable walletFacet;

    event WalletCreated(address indexed walletAddress, address indexed owner);

    constructor(
        address _diamondCutFacet,
        address _diamondLoupeFacet,
        address _ownershipFacet,
        address _walletFacet
    ) {
        diamondCutFacet = _diamondCutFacet;
        diamondLoupeFacet = _diamondLoupeFacet;
        ownershipFacet = _ownershipFacet;
        walletFacet = _walletFacet;
    }

    function createWallet(address _owner) external returns (address) {
        // Deploy the Diamond, setting the factory (this) as the temporary owner
        // so we can cut the facets in.
        Diamond d = new Diamond(address(this), diamondCutFacet);

        // Prepare the cut
        IDiamondCut.FacetCut[] memory cut = new IDiamondCut.FacetCut[](3);

        // Loupe Facet
        bytes4[] memory loupeSelectors = new bytes4[](4);
        loupeSelectors[0] = IDiamondLoupe.facets.selector;
        loupeSelectors[1] = IDiamondLoupe.facetFunctionSelectors.selector;
        loupeSelectors[2] = IDiamondLoupe.facetAddresses.selector;
        loupeSelectors[3] = IDiamondLoupe.facetAddress.selector;
        cut[0] = IDiamondCut.FacetCut({
            facetAddress: diamondLoupeFacet,
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: loupeSelectors
        });

        // Ownership Facet
        bytes4[] memory ownershipSelectors = new bytes4[](2);
        ownershipSelectors[0] = IERC173.transferOwnership.selector;
        ownershipSelectors[1] = IERC173.owner.selector;
        cut[1] = IDiamondCut.FacetCut({
            facetAddress: ownershipFacet,
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: ownershipSelectors
        });

        // Wallet Facet
        // We need to manually calculate the selector for executeCall(address,uint256,bytes)
        // bytes4(keccak256("executeCall(address,uint256,bytes)"))
        bytes4[] memory walletSelectors = new bytes4[](1);
        walletSelectors[0] = bytes4(keccak256("executeCall(address,uint256,bytes)"));
        cut[2] = IDiamondCut.FacetCut({
            facetAddress: walletFacet,
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: walletSelectors
        });

        // Execute the cut
        IDiamondCut(address(d)).diamondCut(cut, address(0), "");

        // Transfer ownership to the actual owner
        IERC173(address(d)).transferOwnership(_owner);

        emit WalletCreated(address(d), _owner);
        return address(d);
    }
}
