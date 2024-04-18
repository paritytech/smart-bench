import click
import json
from web3 import Web3, Account

@click.command()
def main():
    keys = [
        # "5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133", #alith
        "8075991ce870b93a8870eca0c0f91913d12f47948ca0fd25b49c6fa7cdbeee8b" #balthazar
    ]
    for i in range(1500): # 1500 other accounts
        padded_hex = f"{i:064}"
        keys.append(padded_hex)


    w3 = Web3()

    balances = []
    for private_key in keys:
        try:
            account = Account.from_key(private_key)
            balances.append([account.address, 110000000000000000000])
        except ValueError as e:
            click.echo(f"Error: {e}")
            return

    print(json.dumps(balances))

if __name__ == "__main__":
    main()
