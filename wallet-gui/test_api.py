import requests
import json

# Test the API endpoints
api_base = "https://time-coin.io"

print("Testing API endpoints...")
print("\n1. Testing /api/peers:")
try:
    response = requests.get(f"{api_base}/api/peers", timeout=10)
    print(f"Status: {response.status_code}")
    print(f"Response: {json.dumps(response.json(), indent=2)}")
except Exception as e:
    print(f"Error: {e}")

print("\n2. Testing /api/blockchain/info:")
try:
    response = requests.get(f"{api_base}/api/blockchain/info", timeout=10)
    print(f"Status: {response.status_code}")
    print(f"Response: {json.dumps(response.json(), indent=2)}")
except Exception as e:
    print(f"Error: {e}")

print("\n3. Testing other possible endpoints:")
for endpoint in ["/api/blockchain", "/api/info", "/api/block/height", "/api/getblockcount"]:
    try:
        response = requests.get(f"{api_base}{endpoint}", timeout=10)
        print(f"\n{endpoint}: Status {response.status_code}")
        if response.status_code == 200:
            print(f"Response: {json.dumps(response.json(), indent=2)}")
    except Exception as e:
        print(f"{endpoint}: Error - {e}")
