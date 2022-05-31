pragma solidity ^0.8.0;

import "node_modules/@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract BenchERC20 is ERC20 {
    constructor() ERC20("BenchERC20", "CAN") {
    }
}