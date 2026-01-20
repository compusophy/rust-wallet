// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @notice Minimal ERC721 implementation for Identity NFT
contract IdentityNFT {
    string public name = "IdentityNFT";
    string public symbol = "ID";
    
    mapping(uint256 => address) public ownerOf;
    mapping(address => uint256) public balanceOf;
    
    uint256 public nextTokenId = 1;
    
    event Transfer(address indexed from, address indexed to, uint256 indexed tokenId);

    function mint() external returns (uint256) {
        uint256 tokenId = nextTokenId++;
        _mint(msg.sender, tokenId);
        return tokenId;
    }

    function _mint(address to, uint256 tokenId) internal {
        require(to != address(0), "INVALID_RECIPIENT");
        require(ownerOf[tokenId] == address(0), "ALREADY_MINTED");

        balanceOf[to]++;
        ownerOf[tokenId] = to;

        emit Transfer(address(0), to, tokenId);
    }
    
    function tokenURI(uint256 id) external view returns (string memory) {
        return ""; // Todo: Metadata
    }
    
    function supportsInterface(bytes4 interfaceId) external pure returns (bool) {
        return interfaceId == 0x80ac58cd // ERC721
            || interfaceId == 0x01ffc9a7; // ERC165
    }
}
