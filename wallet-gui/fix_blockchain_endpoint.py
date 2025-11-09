with open('src/network.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix the URL - it should be /blockchain/info not /api/blockchain/info
old_url = 'let url = format!("{}/api/blockchain/info", self.api_endpoint);'
new_url = 'let url = format!("{}/blockchain/info", self.api_endpoint);'

content = content.replace(old_url, new_url)

with open('src/network.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed blockchain info endpoint!')
