async function main() {
    // We get the contract to deploy
    const BenchERC20 = await ethers.getContractFactory("BenchERC20");
    const erc20 = await BenchERC20.deploy(1000);

    await erc20.deployed();

    console.log("BenchERC20 deployed to:", erc20.address);
}

main()
    .then(() => process.exit(0))
    .catch((error) => {
        console.error(error);
        process.exit(1);
    });