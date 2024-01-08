require('@nomiclabs/hardhat-ethers');

const { alith } = require('./secrets.json');

/**
 * @type import('hardhat/config').HardhatUserConfig
 */
module.exports = {
  defaultNetwork: "dev",
  networks: {
    dev: {
      url: "http://127.0.0.1:9933",
      chainId: 1281,
      accounts: [alith]
    },
  },
  solidity: "0.8.1",
  paths: {
    sources: "./contracts",
    tests: "./test",
    cache: "./cache",
    artifacts: "./evm"
  },
};
