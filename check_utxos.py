import socket, json

def rpc(host, method, params):
    body = json.dumps({'jsonrpc':'2.0','id':'1','method':method,'params':params})
    req = f'POST / HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {len(body)}\r\n\r\n{body}'
    s = socket.socket()
    s.settimeout(5)
    s.connect((host, 24101))
    s.sendall(req.encode())
    data = b''
    while True:
        try:
            chunk = s.recv(4096)
            if not chunk: break
            data += chunk
        except: break
    s.close()
    resp = data.decode().split('\r\n\r\n',1)[1]
    return json.loads(resp)

addr = 'TIME0AsqaMhkNMxinWmnNzqBky7LHEiaUoRYQE'
for host in ['50.28.104.50', '64.91.241.10', '165.232.154.150']:
    print(f"\n=== {host} ===")
    try:
        r = rpc(host, 'listunspentmulti', [[addr]])
        utxos = r.get('result', []) or []
        print(f"UTXOs: {len(utxos)}")
        for u in utxos:
            print(f"  txid={u['txid'][:16]}... vout={u['vout']} amount={u['amount']} spendable={u.get('spendable')} state={u.get('state')}")
        r2 = rpc(host, 'getbalance', [addr])
        print(f"Balance: {json.dumps(r2.get('result'))}")
    except Exception as e:
        print(f"Error: {e}")
