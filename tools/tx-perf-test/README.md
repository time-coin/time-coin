# Transaction Performance Test Tool

A comprehensive tool to measure real transaction verification speed on the TIME Coin network.

## Features

- **Real transaction submission**: Sends actual signed transactions to the network
- **Performance metrics**: Tracks submission time, success rate, and throughput
- **Detailed reporting**: Provides comprehensive statistics with JSON export
- **Configurable parameters**: Control transaction count, amount, fee, and timing
- **Network support**: Works with mainnet, testnet, and devnet
- **Testnet coin generation**: Mint test coins for testnet/devnet testing (no need for pre-funded wallets)

## Installation

From the project root:

```bash
cd tools/tx-perf-test
cargo build --release
```

## Quick Start

For a quick demonstration, run one of the example scripts:

**Linux/Mac:**
```bash
chmod +x example.sh
./example.sh
```

**Windows:**
```cmd
example.bat
```

These scripts demonstrate testnet coin generation and various performance test scenarios.

## Usage

### Basic Usage

```bash
cargo run --release -- \
  --private-key <HEX_PRIVATE_KEY> \
  --recipient <RECIPIENT_ADDRESS> \
  --tx-count 100 \
  --amount 1000 \
  --fee 100
```

### Basic Usage with Testnet Coin Generation

For testnet/devnet, you can generate coins automatically:

```bash
cargo run --release -- \
  --private-key <HEX_PRIVATE_KEY> \
  --recipient <RECIPIENT_ADDRESS> \
  --tx-count 100 \
  --amount 1000 \
  --fee 100 \
  --network testnet \
  --mint-coins 1000000
```

This will mint 1,000,000 test coins to your wallet before running the performance test.

### Full Options

```bash
cargo run --release -- \
  --api-url http://localhost:3030 \
  --private-key <HEX_PRIVATE_KEY> \
  --recipient <RECIPIENT_ADDRESS> \
  --tx-count 100 \
  --amount 1000 \
  --fee 100 \
  --delay-ms 100 \
  --network testnet \
  --output results.json \
  --verbose
```

### Parameters

| Parameter | Short | Description | Default |
|-----------|-------|-------------|---------|
| `--api-url` | `-a` | API endpoint URL | `http://localhost:3030` |
| `--private-key` | `-p` | Sender's private key (hex) | *Required* |
| `--recipient` | `-r` | Recipient address | *Required* |
| `--tx-count` | `-n` | Number of transactions | `10` |
| `--amount` | `-a` | Amount per transaction | `1000` |
| `--fee` | `-f` | Fee per transaction | `100` |
| `--delay-ms` | `-d` | Delay between transactions (ms) | `100` |
| `--network` | | Network type (mainnet/testnet/devnet) | `testnet` |
| `--output` | `-o` | Output JSON file path | *None* |
| `--verbose` | `-v` | Verbose output | `false` |
| `--mint-coins` | | Generate testnet coins (testnet/devnet only) | `0` |

## Example Output

```
🚀 TIME Coin Transaction Performance Test
==========================================
API Endpoint:    http://localhost:3030
Transaction Count: 100
Amount per TX:   1000 TIME
Fee per TX:      100 TIME
Delay:           100 ms
Network:         testnet

✅ Wallet loaded: t1abc123...
   Balance: 0 TIME
💰 Generating 500000 testnet coins...
✅ Generated 500000 coins (new balance: 500000)

📤 Sending transactions...

  Sent 10/100 transactions...
  Sent 20/100 transactions...
  ...
  Sent 100/100 transactions...

📊 Performance Report
==========================================
Total transactions:   100
Successful:           98
Failed:               2
Total duration:       15234 ms
Average submit time:  142.35 ms
Min submit time:      95 ms
Max submit time:      387 ms
Throughput:           6.43 TPS

✅ Report saved to: results.json
✅ Performance test completed successfully
```

## JSON Report Format

The tool can export detailed results to JSON:

```json
{
  "total_transactions": 100,
  "successful_transactions": 98,
  "failed_transactions": 2,
  "total_duration_ms": 15234,
  "average_submission_time_ms": 142.35,
  "min_submission_time_ms": 95,
  "max_submission_time_ms": 387,
  "transactions_per_second": 6.43,
  "test_started": "2025-11-16T15:30:00Z",
  "test_completed": "2025-11-16T15:30:15Z",
  "transactions": [
    {
      "tx_id": "a3f5...",
      "tx_number": 1,
      "amount": 1000,
      "fee": 100,
      "submission_time": "2025-11-16T15:30:00Z",
      "submission_duration_ms": 143,
      "success": true,
      "error": null
    },
    ...
  ]
}
```

## Test Scenarios

### 0. Generate Test Funds (Testnet Only)

Generate test coins without needing a pre-funded wallet:

```bash
cargo run --release -- \
  -p <KEY> -r <ADDR> \
  --network testnet \
  --mint-coins 10000000 \
  --tx-count 1
```

### 1. Burst Test (Maximum Throughput)

Test how many transactions can be submitted rapidly:

```bash
cargo run --release -- \
  -p <KEY> -r <ADDR> \
  --network testnet \
  --mint-coins 5000000 \
  --tx-count 1000 \
  --delay-ms 0
```

### 2. Sustained Load Test

Test consistent transaction flow:

```bash
cargo run --release -- \
  -p <KEY> -r <ADDR> \
  --network testnet \
  --mint-coins 3000000 \
  --tx-count 500 \
  --delay-ms 1000
```

### 3. Small Transaction Test

Test with minimal amounts:

```bash
cargo run --release -- \
  -p <KEY> -r <ADDR> \
  --network testnet \
  --mint-coins 500000 \
  --tx-count 100 \
  --amount 1 \
  --fee 1
```

### 4. Large Transaction Test

Test with larger amounts:

```bash
cargo run --release -- \
  -p <KEY> -r <ADDR> \
  --network testnet \
  --mint-coins 10000000 \
  --tx-count 50 \
  --amount 100000 \
  --fee 1000
```

## Requirements

- A wallet with private key (for testnet, can use `--mint-coins` to generate funds)
- Running TIME Coin node with API enabled (for mainnet tests)
- Network connectivity to the node (for mainnet tests)

**Note**: For testnet/devnet, you no longer need a pre-funded wallet! Use the `--mint-coins` flag to generate test funds.

## Metrics Explained

- **Total duration**: Wall clock time from first to last transaction
- **Submission time**: Time to create, sign, and submit each transaction
- **Success rate**: Percentage of transactions accepted by the network
- **TPS (Transactions Per Second)**: Throughput calculated as successful txs / total duration

## Troubleshooting

### Insufficient Funds Error

For testnet/devnet, use the `--mint-coins` flag to generate test funds:
```bash
--mint-coins 10000000  # Generates 10 million test coins
```

For mainnet, ensure your wallet has enough balance:
```
Required = (amount + fee) × tx_count
```

### Connection Refused

Verify the API endpoint is correct and the node is running:
```bash
curl http://localhost:3030/health
```

### Transaction Failures

Check the verbose output to see specific error messages:
```bash
cargo run --release -- <params> --verbose
```

## Notes

- This tool sends **real transactions** on mainnet that will spend actual coins
- For testnet/devnet, use `--mint-coins` to generate test funds without needing a pre-funded wallet
- The `--mint-coins` flag only works with testnet and devnet (blocked on mainnet for safety)
- Make sure you have sufficient balance before running mainnet tests
- Use testnet for experimentation and testing
- The tool includes basic rate limiting with the `--delay-ms` parameter
- Consider network latency when interpreting submission times

## License

MIT License - See LICENSE file for details
