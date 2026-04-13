from typing import Any

class PyPiClient:
    """
    An asynchronous client for fetching Python package information from the PyPi API.
    """
    def __new__(cls) -> "PyPiClient": ...

    async def get(self, package: str) -> PyPiPackage:
        """
        Fetches a single python package from PyPi asynchronously.

        >>> await PyPiClient().get("requests")
        PyPiPackage(name='requests', releases={...})

        Args:
            package (str): The name of the package to fetch.

        Returns:
            PyPiPackage: An instance of PyPiPackage.
        """
        ...

    async def get_many(self, packages: list[str], max_concurrency: int | None = None) -> list[PyPiPackage]:
        """
        Fetches multiple python packages from PyPi asynchronously. If `max_concurrency` is not provided, it will default to 250.
        The `max_concurrency` parameter is used to limit the number of concurrent requests to avoid overwhelming the PyPi API
        and the local system. A range of `(1 - 1,000)` is recommended, with `250` being a good default for most use cases.
        Adjust this number based on your system capabilities and the expected load.

        >>> await PyPiClient().get_many(["requests", "numpy", ...])
        [
            PyPiPackage(name='requests', releases={...}),
            PyPiPackage(name='numpy', releases={...}),
            ...
        ]
        Args:
            packages (list[str]): A list of package names to fetch.
            max_concurrency (int | None): The maximum number of concurrent requests.

        Returns:
            list[PyPiPackage]: A list of PyPiPackage instances.
        """
        ...

class PyPiPackage:
    """
    A class representing a Python package from PyPi. It contains the package name and all of the contents from the `"releases"`
    field of the PyPi API response for that package.
    """
    name: str
    releases: dict[str, Any]
