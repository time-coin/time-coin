with open('src/network.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Check if the methods already exist
if 'pub fn current_block_height(&self) -> u64' not in content:
    # Find the api_endpoint getter and add after it
    insertion_point = '    pub fn api_endpoint(&self) -> &str {\n        &self.api_endpoint\n    }'
    
    new_methods = '''    pub fn api_endpoint(&self) -> &str {
        &self.api_endpoint
    }
    
    pub fn current_block_height(&self) -> u64 {
        self.current_block_height
    }
    
    pub fn network_block_height(&self) -> u64 {
        self.network_block_height
    }'''
    
    content = content.replace(insertion_point, new_methods)

# Also make sure the struct has the fields
if 'current_block_height: u64,' not in content:
    # Find the struct definition and add fields
    old_fields = '''    is_syncing: bool,
    sync_progress: f32,
}'''
    
    new_fields = '''    is_syncing: bool,
    sync_progress: f32,
    current_block_height: u64,
    network_block_height: u64,
}'''
    
    content = content.replace(old_fields, new_fields)

# Update the new() constructor if needed
if 'current_block_height: 0,' not in content:
    old_new_body = '''        Self {
            api_endpoint,
            connected_peers: Vec::new(),
            is_syncing: false,
            sync_progress: 0.0,
        }'''
    
    new_new_body = '''        Self {
            api_endpoint,
            connected_peers: Vec::new(),
            is_syncing: false,
            sync_progress: 0.0,
            current_block_height: 0,
            network_block_height: 0,
        }'''
    
    content = content.replace(old_new_body, new_new_body)

with open('src/network.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed network.rs with block height methods!')

# Let's verify the file
with open('src/network.rs', 'r', encoding='utf-8') as f:
    content = f.read()
    if 'pub fn current_block_height' in content:
        print('✓ current_block_height method exists')
    else:
        print('✗ current_block_height method missing')
    
    if 'pub fn network_block_height' in content:
        print('✓ network_block_height method exists')
    else:
        print('✗ network_block_height method missing')
    
    if 'current_block_height: u64' in content:
        print('✓ current_block_height field exists')
    else:
        print('✗ current_block_height field missing')

