// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Script.sol";
import "../src/facets/DiamondCutFacet.sol";
import "../src/facets/DiamondLoupeFacet.sol";
import "../src/facets/OwnershipFacet.sol";
import "../src/facets/WalletFacet.sol";
import "../src/WalletFactory.sol";

contract DeployScript is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        // 1. Deploy Facets
        DiamondCutFacet dCut = new DiamondCutFacet();
        console.log("DiamondCutFacet deployed:", address(dCut));

        DiamondLoupeFacet dLoupe = new DiamondLoupeFacet();
        console.log("DiamondLoupeFacet deployed:", address(dLoupe));

        OwnershipFacet ownership = new OwnershipFacet();
        console.log("OwnershipFacet deployed:", address(ownership));

        WalletFacet wallet = new WalletFacet();
        console.log("WalletFacet deployed:", address(wallet));

        // 2. Deploy Factory
        WalletFactory factory = new WalletFactory(
            address(dCut),
            address(dLoupe),
            address(ownership),
            address(wallet)
        );
        console.log("WalletFactory deployed:", address(factory));

        vm.stopBroadcast();
    }
}
