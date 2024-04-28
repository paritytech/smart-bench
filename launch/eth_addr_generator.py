import click
import json
from web3 import Web3, Account

@click.command()
def main():
    seeds = [
        # "5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133", #alith
        "8075991ce870b93a8870eca0c0f91913d12f47948ca0fd25b49c6fa7cdbeee8b" #balthazar
    ]
    for i in range(5000): # 5000 other accounts
        padded_hex = f"{i:064}"
        seeds.append(padded_hex)


    w3 = Web3()

    balances = []
    for seed in seeds:
        try:
            account = Account.from_key(seed)
            balances.append([account.address, 110000000000000000000])
        except ValueError as e:
            click.echo(f"Error: {e}")
            return

    print(json.dumps(balances, indent=4))

if __name__ == "__main__":
    main()
