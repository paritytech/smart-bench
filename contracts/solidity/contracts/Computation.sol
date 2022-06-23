pragma solidity ^0.8.0;

contract Computation {

    constructor() { }

    function oddProduct(int32 x) public pure returns (int64) {
        int64 prod = 1;
        for (int32 counter = 1; counter <= x; counter++) {
            unchecked {
                prod *= 2 * counter - 1;
            }
        }
        return prod;
    }

    function triangleNumber(int32 x) public pure returns (int64) {
        int64 sum = 0;
        for (int32 counter = 1; counter <= x; counter++) {
            unchecked {
                sum += counter;
            }
        }
        return sum;
    }
}