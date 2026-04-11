import re

files = ['array.rs', 'io.rs', 'map.rs', 'set.rs', 'primitives.rs']

for f in files:
    path = f'C:\\Users\\x\\dev\\ts-native\\crates\\tsn-vm\\src\\intrinsic\\{f}'
    with open(path, 'r') as fh:
        content = fh.read()
    
    # Add op macro import
    if 'use tsn_op_macros::op;' not in content:
        content = content.replace('use tsn_types', 'use tsn_op_macros::op;\nuse tsn_types', 1)
    
    # Add #[op("x")] before each pub fn (but not before existing #[op] or OPS arrays)
    content = re.sub(r'^(\s*)(pub fn )', r'\1#[op("x")]\n\1\2', content, flags=re.MULTILINE)
    
    with open(path, 'w', newline='\r\n') as fh:
        fh.write(content)
    
    print(f'Fixed {f}')
