import requests

peers = [
    "216.198.79.65",
    "64.29.17.65", 
    "161.35.129.70",
    "178.128.199.144",
    "165.232.154.150"
]

print("Testing peers on RPC port 24101 (Testnet):")
for peer_ip in peers:
    url = f"http://{peer_ip}:24101/blockchain/info"
    try:
        print(f"\n{peer_ip}:24101")
        response = requests.get(url, timeout=5)
        if response.status_code == 200:
            data = response.json()
            print(f"  SUCCESS! Height: {data.get('height', 'N/A')}")
            print(f"  Network: {data.get('network', 'N/A')}")
        else:
            print(f"  HTTP {response.status_code}")
    except Exception as e:
        print(f"  Failed: {str(e)[:50]}")
