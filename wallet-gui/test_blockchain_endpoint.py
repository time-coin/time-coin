import requests
import json

api_base = "https://time-coin.io"

print("Testing blockchain info endpoint...")
print(f"\nEndpoint: {api_base}/blockchain/info")

try:
    response = requests.get(f"{api_base}/blockchain/info", timeout=10)
    print(f"Status Code: {response.status_code}")
    
    if response.status_code == 200:
        print("\nSuccess! Response:")
        data = response.json()
        print(json.dumps(data, indent=2))
    else:
        print(f"Failed with status {response.status_code}")
        print(f"Response text: {response.text}")
        
except Exception as e:
    print(f"Error: {e}")

print("\n\nAlso testing with port 24101 (node API port)...")
print(f"Endpoint: {api_base}:24101/blockchain/info")

try:
    response = requests.get(f"{api_base}:24101/blockchain/info", timeout=10)
    print(f"Status Code: {response.status_code}")
    
    if response.status_code == 200:
        print("\nSuccess! Response:")
        data = response.json()
        print(json.dumps(data, indent=2))
    else:
        print(f"Failed with status {response.status_code}")
        
except Exception as e:
    print(f"Error: {e}")
