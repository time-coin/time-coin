import requests

# The main TimeCoin API endpoint
api_endpoint = "https://timecoin.online/api"

print("Testing TimeCoin API endpoint:")
try:
    # Test peers endpoint
    url = f"{api_endpoint}/network/peers"
    print(f"\nTesting: {url}")
    response = requests.get(url, timeout=10)
    if response.status_code == 200:
        data = response.json()
        print(f"  Success! Found {data.get('count', 0)} peers")
        print(f"  First few peers: {data.get('peers', [])[:3]}")
    
    # Test blockchain info endpoint
    url = f"{api_endpoint}/blockchain/info"
    print(f"\nTesting: {url}")
    response = requests.get(url, timeout=10)
    if response.status_code == 200:
        data = response.json()
        print(f"  Success!")
        print(f"  Height: {data.get('height', 'unknown')}")
        print(f"  Network: {data.get('network', 'unknown')}")
        print(f"  Best block: {data.get('best_block_hash', 'unknown')[:16]}...")
    else:
        print(f"  Status: {response.status_code}")
except Exception as e:
    print(f"  Error: {e}")
