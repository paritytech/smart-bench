pragma solidity ^0.8.0;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";

contract BenchERC721 is ERC721 {
    constructor() ERC721("BenchERC721", "CANFT") {
    }

    function mint(uint256 tokenId) public {
        _mint(msg.sender, tokenId);
    }
}