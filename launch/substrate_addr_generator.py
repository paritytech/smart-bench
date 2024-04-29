import click
import json
from substrateinterface import Keypair

@click.command()
def main():
    seeds = [
        "//Alice",
        "//Bob",
    ]
    for i in range(5000): # 5000 other accounts
        seeds.append(f"//Sender/{i}")

    balances = []
    for seed in seeds:
        try:
            keypair = Keypair.create_from_uri(suri=seed)
            address = keypair.ss58_address

            balances.append([address, 110000000000000000000])
        except ValueError as e:
            click.echo(f"Error: {e}")
            return

    print(json.dumps(balances, indent=4))

if __name__ == "__main__":
    main()
