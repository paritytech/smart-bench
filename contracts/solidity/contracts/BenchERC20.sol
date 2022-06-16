pragma solidity ^0.8.0;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract BenchERC20 is ERC20 {
    constructor(uint256 initialSupply) ERC20("BenchERC20", "CAN") {
        _mint(msg.sender, initialSupply);
    }
}