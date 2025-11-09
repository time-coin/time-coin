import requests
import json

# Get the list of peers first
print("1. Getting peer list from website...")
response = requests.get("https://time-coin.io/api/peers", timeout=10)
peers = response.json()
print(f"Found {len(peers)} peers:")
for peer in peers:
    print(f"  - {peer}")

print("\n2. Testing /blockchain/info on each peer...")
for peer_addr in peers[:3]:  # Test first 3 peers
    peer_ip = peer_addr.split(':')[0]
    url = f"http://{peer_ip}:24101/blockchain/info"
    print(f"\nTesting: {url}")
    
    try:
        response = requests.get(url, timeout=5)
        print(f"  Status: {response.status_code}")
        
        if response.status_code == 200:
            data = response.json()
            print(f"  Height: {data.get('height')}")
            print(f"  Network: {data.get('network')}")
            print(f"  Hash: {data.get('best_block_hash', '')[:20]}...")
            print(f"  Supply: {data.get('total_supply')}")
        else:
            print(f"  Failed: {response.text[:100]}")
            
    except Exception as e:
        print(f"  Error: {e}")
