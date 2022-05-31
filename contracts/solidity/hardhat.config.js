require('@nomiclabs/hardhat-ethers');

const { alith } = require('./secrets.json');

/**
 * @type import('hardhat/config').HardhatUserConfig
 */
module.exports = {
  defaultNetwork: "dev",
  networks: {
    dev: {
      dev: {
        url: "http://127.0.0.1:9933",
        chainId: 1281,
        accounts: [alith]
      },
    }
  },
  solidity: "0.8.0",
  paths: {
    sources: "./contracts",
    tests: "./test",
    cache: "./cache",
    artifacts: "./artifacts"
  },
};
