import asyncio
import pandas as pd

from viper import PyPiPackage, PyPiClient

async def main():
    df = pd.read_csv("top-pypi-packages.csv")
    urls = [p for p in df["project"].tolist() if isinstance(p, str)]

    client = PyPiClient()
    package: PyPiPackage = await client.get("requests")
    print(f"fetched a single package: {package.name}")

    _ = await client.get_many(urls[:10000])


if __name__ == "__main__":
    asyncio.run(main())