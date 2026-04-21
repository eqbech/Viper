# Viper 🐍

A high-performance PyPI package client built with Rust and Python. Viper provides fast, concurrent access to PyPI package metadata with full async/await support.

## Features

- **High Performance**: Rust backend compiled to Python C extension for blazing-fast performance
- **Async Support**: Full async/await support via tokio runtime
- **Concurrent Fetching**: Fetch multiple packages concurrently with configurable rate limiting
- **Progress Tracking**: Visual progress bar for batch operations
- **Type-Safe JSON Parsing**: Uses serde-query for efficient JSON deserialization
- **Comprehensive Logging**: Built-in logging for debugging and monitoring
- **DNS Resolution**: Custom Hickory DNS resolver for improved reliability

## Installation

The package is available as a pre-built wheel for Python 3.8+:

```bash
pip install lib  # Install from built wheel
```

## Quick Start

### Single Package Fetch

```python
import asyncio
from viper import PyPiClient

async def main():
    client = PyPiClient()
    package = await client.get("requests")
    print(f"Package name: {package.name}")

asyncio.run(main())
```

### Batch Package Fetch

```python
import asyncio
from viper import PyPiClient

async def main():
    client = PyPiClient()
    packages = ["requests", "django", "flask", "numpy", "pandas"]
    results = await client.get_many(packages, max_concurrency=50)
    print(f"Fetched {len(results)} packages")

asyncio.run(main())
```

## API Reference

### `PyPiClient`

Main client for fetching packages from PyPI.

#### Methods

##### `__init__()`
Creates a new PyPI client instance. Initializes the logger and internal HTTP client.

##### `get(package: str) -> Coroutine[PyPiPackage]`
Fetch metadata for a single package.

**Parameters:**
- `package` (str): The name of the package to fetch

**Returns:** Coroutine that resolves to a `PyPiPackage` object

**Example:**
```python
package = await client.get("requests")
print(package.name)  # "requests"
```

##### `get_many(packages: List[str], max_concurrency: Optional[int] = None) -> Coroutine[List[PyPiPackage]]`
Fetch metadata for multiple packages concurrently.

**Parameters:**
- `packages` (List[str]): List of package names to fetch
- `max_concurrency` (Optional[int]): Maximum number of concurrent requests (default: 250)

**Returns:** Coroutine that resolves to a list of successfully fetched `PyPiPackage` objects

**Example:**
```python
results = await client.get_many(package_list, max_concurrency=100)
for pkg in results:
    print(pkg.name)
```

### `PyPiPackage`

Represents a PyPI package with its metadata.

**Attributes:**
- `name` (str): The name of the package

## Performance Considerations

- **Default Concurrency**: Set to 250 concurrent requests by default
- **No PyPI Rate Limiting**: PyPI appears to have no strict rate limiting, but concurrency is capped to avoid overwhelming your system
- **Memory Usage**: The entire JSON response is loaded into memory before parsing. For future improvements, streaming parsing could be implemented
- **Data Payload**: The endpoint returns more data than necessary. Fetching only essential metadata might be more efficient

## Building from Source

### Prerequisites

- Python 3.8 or higher
- Rust toolchain (1.70+)
- maturin (1.12+)

### Build Steps

1. Clone the repository and navigate to the directory
2. Create a virtual environment:
   ```bash
   python -m venv .venv
   source .venv/bin/activate  # On Windows: .venv\Scripts\activate
   ```

3. Install build dependencies:
   ```bash
   pip install maturin pandas
   ```

4. Build the extension:
   ```bash
   maturin develop  # For development (in-place)
   # or
   maturin build --release  # For wheel distribution
   ```

## Dependencies

### Rust Dependencies
- `pyo3` - Python bindings for Rust
- `tokio` - Async runtime
- `reqwest` - HTTP client
- `serde_json` - JSON parsing
- `serde-query` - Type-safe JSON querying
- `indicatif` - Progress bars
- `log` / `env_logger` - Logging

### Python Dependencies
- `pandas` (2.0.3+)

## Limitations

- Only fetches basic package metadata (name and releases)
- Full JSON response is loaded into memory
- No fine-grained error handling (failed packages are returned in a list but without specific error reasons)

## Testing

Run the included test script to verify functionality:

```bash
python test.py
```

This will fetch packages from the top PyPI packages CSV file using the batch API.
