#!/bin/bash
# Update patches.json with current patch files metadata
# Usage: ./update-patches-json.sh

PATCHES_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JSON_FILE="$PATCHES_DIR/patches.json"
REPO_URL="https://raw.githubusercontent.com/Akaere-NetWorks/whois-server/refs/heads/main/patches"

echo "Updating patches.json..."
echo "Scanning for .patch files in: $PATCHES_DIR"

# Start building JSON
patches_array=""
patch_count=0

for patch_file in "$PATCHES_DIR"/*.patch; do
    if [ -f "$patch_file" ]; then
        patch_count=$((patch_count + 1))
        filename=$(basename "$patch_file")
        
        # Calculate SHA1
        if command -v sha1sum &> /dev/null; then
            sha1=$(sha1sum "$patch_file" | awk '{print $1}')
        elif command -v shasum &> /dev/null; then
            sha1=$(shasum "$patch_file" | awk '{print $1}')
        else
            echo "Error: sha1sum or shasum not found"
            exit 1
        fi
        
        # Get file size
        size=$(wc -c < "$patch_file" | tr -d ' ')
        
        # Get modification time
        modified=$(date -Iseconds -r "$patch_file" 2>/dev/null || stat -c %y "$patch_file" | cut -d. -f1 | sed 's/ /T/')
        
        # Extract description from second comment line
        description=$(head -n 2 "$patch_file" | tail -n 1 | sed 's/^# //')
        
        echo "  - Found: $filename (SHA1: $sha1, Size: $size bytes)"
        
        # Add comma if not first item
        if [ $patch_count -gt 1 ]; then
            patches_array="$patches_array,"
        fi
        
        # Add patch entry
        patches_array="$patches_array
    {
      \"name\": \"$filename\",
      \"description\": \"$description\",
      \"url\": \"$REPO_URL/$filename\",
      \"sha1\": \"$sha1\",
      \"size\": $size,
      \"enabled\": true,
      \"priority\": $patch_count,
      \"modified\": \"$modified\"
    }"
    fi
done

# Generate complete JSON
current_time=$(date -Iseconds)

cat > "$JSON_FILE" << 'EOF_TEMPLATE'
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "version": "1.0.0",
  "description": "WHOIS Server Patch Metadata - For online updates and integrity verification",
  "repository": "https://github.com/Akaere-NetWorks/whois-server",
  "last_updated": "LAST_UPDATED_PLACEHOLDER",
  "patches": [PATCHES_ARRAY_PLACEHOLDER
  ],
  "metadata": {
    "format_version": "1.0",
    "checksum_algorithm": "SHA1",
    "update_url": "REPO_URL_PLACEHOLDER/patches.json",
    "documentation": "https://github.com/Akaere-NetWorks/whois-server/blob/main/patches/README.md"
  }
}
EOF_TEMPLATE

# Replace placeholders
sed -i "s|LAST_UPDATED_PLACEHOLDER|$current_time|g" "$JSON_FILE"
sed -i "s|REPO_URL_PLACEHOLDER|$REPO_URL|g" "$JSON_FILE"

# Replace patches array (using a temporary file due to multiline content)
temp_file=$(mktemp)
awk -v patches="$patches_array" '{gsub(/PATCHES_ARRAY_PLACEHOLDER/, patches); print}' "$JSON_FILE" > "$temp_file"
mv "$temp_file" "$JSON_FILE"

echo ""
echo "✓ Updated $JSON_FILE"
echo "✓ Total patches: $patch_count"
echo ""
echo "You can now commit and push the changes:"
echo "  git add patches/patches.json"
echo "  git commit -m 'Update patches.json metadata'"
