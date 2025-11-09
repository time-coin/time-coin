import socket
import requests

peers = [
    "216.198.79.65",
    "64.29.17.65", 
    "161.35.129.70",
    "178.128.199.144",
    "165.232.154.150"
]

# Common RPC ports to try
ports_to_test = [24101, 8332, 8545, 9650, 24133]

print("Testing which ports are open on peers:")
for peer_ip in peers:
    print(f"\nPeer: {peer_ip}")
    for port in ports_to_test:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(2)
        result = sock.connect_ex((peer_ip, port))
        sock.close()
        
        if result == 0:
            print(f"  Port {port}: OPEN")
            # Try to get blockchain info
            for endpoint in ['/blockchain/info', '/info', '/api/blockchain/info', '/rpc']:
                try:
                    url = f"http://{peer_ip}:{port}{endpoint}"
                    response = requests.get(url, timeout=2)
                    if response.status_code == 200:
                        print(f"    -> {endpoint} returned: {response.text[:100]}")
                        break
                except:
                    pass
        else:
            print(f"  Port {port}: closed")
