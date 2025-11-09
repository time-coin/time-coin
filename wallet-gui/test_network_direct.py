import requests
import sys

peers = [
    "216.198.79.65",
    "64.29.17.65", 
    "161.35.129.70",
    "178.128.199.144",
    "165.232.154.150"
]

print("Testing peer endpoints directly:")
for peer_ip in peers:
    url = f"http://{peer_ip}:24101/blockchain/info"
    try:
        print(f"\nTesting: {url}")
        response = requests.get(url, timeout=5)
        if response.status_code == 200:
            data = response.json()
            print(f"  ✓ Height: {data.get('height', 'unknown')}")
            print(f"  Network: {data.get('network', 'unknown')}")
            print(f"  Response: {data}")
        else:
            print(f"  ✗ Status: {response.status_code}")
    except Exception as e:
        print(f"  ✗ Error: {e}")
