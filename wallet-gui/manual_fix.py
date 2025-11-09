with open('src/main.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

# Print the actual problematic sections
print("=== Section 1 (lines 145-170) ===")
for i in range(144, min(171, len(lines))):
    print(f"{i+1}: {lines[i]}", end='')

print("\n\n=== Section 2 (lines 200-225) ===")
for i in range(199, min(226, len(lines))):
    print(f"{i+1}: {lines[i]}", end='')

# Now let's do a line-by-line fix
new_lines = []
skip_until = -1

for i, line in enumerate(lines):
    if i < skip_until:
        continue
    
    # Pattern 1: Look for "if let Ok(temp_net) = result {" which is wrong
    if i < len(lines) - 3 and 'if let Ok(temp_net) = result {' in line:
        # This is the wrong pattern, skip the next 3 lines too
        skip_until = i + 4  # Skip this line and next 3
        continue
    
    # Pattern 2: Fix lines that have "// Update the shared network manager with results"
    if '// Update the shared network manager with results' in line:
        # Skip this line entirely
        continue
    
    new_lines.append(line)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.writelines(new_lines)

print("\n\nRemoved problematic lines")
