import requests
import json

api_base = "https://time-coin.io"

print("Testing API endpoints...")

endpoints = [
    "/api",
    "/api/status",
    "/api/network",
    "/api/network/info",
    "/api/stats",
    "/api/blocks",
    "/api/block/latest",
    "/api/block/current",
    "/api/height",
    "/api/blockheight",
    "/api/chain/info",
    "/api/chain/height",
]

for endpoint in endpoints:
    try:
        response = requests.get(f"{api_base}{endpoint}", timeout=10)
        if response.status_code == 200:
            print(f"\nOK {endpoint}: Status {response.status_code}")
            try:
                data = response.json()
                print(f"Response: {json.dumps(data, indent=2)}")
            except:
                print(f"Response (text): {response.text[:200]}")
        else:
            print(f"FAIL {endpoint}: Status {response.status_code}")
    except Exception as e:
        print(f"ERROR {endpoint}: {e}")
