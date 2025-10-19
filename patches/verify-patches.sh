#!/bin/bash
# Verify patch file integrity against patches.json
# Usage: ./verify-patches.sh [patch-name]

PATCHES_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JSON_FILE="$PATCHES_DIR/patches.json"

# Check if patches.json exists
if [ ! -f "$JSON_FILE" ]; then
    echo "❌ Error: patches.json not found"
    exit 1
fi

# Function to extract JSON field value
get_json_value() {
    local json_file="$1"
    local patch_name="$2"
    local field="$3"
    
    # Simple grep-based extraction (works without jq)
    grep -A 10 "\"name\": \"$patch_name\"" "$json_file" | \
        grep "\"$field\":" | \
        sed 's/.*"'"$field"'": "\?\([^",]*\)"\?.*/\1/' | \
        tr -d ' '
}

# Function to verify a single patch
verify_patch() {
    local patch_name="$1"
    local patch_file="$PATCHES_DIR/$patch_name"
    
    echo "Verifying: $patch_name"
    
    # Check if file exists
    if [ ! -f "$patch_file" ]; then
        echo "  ❌ File not found: $patch_file"
        return 1
    fi
    
    # Get expected SHA1 from JSON
    expected_sha1=$(get_json_value "$JSON_FILE" "$patch_name" "sha1")
    if [ -z "$expected_sha1" ]; then
        echo "  ⚠️  No checksum found in patches.json"
        return 1
    fi
    
    # Calculate actual SHA1
    if command -v sha1sum &> /dev/null; then
        actual_sha1=$(sha1sum "$patch_file" | awk '{print $1}')
    elif command -v shasum &> /dev/null; then
        actual_sha1=$(shasum "$patch_file" | awk '{print $1}')
    else
        echo "  ❌ Error: sha1sum or shasum not found"
        return 1
    fi
    
    # Get expected size
    expected_size=$(get_json_value "$JSON_FILE" "$patch_name" "size")
    actual_size=$(wc -c < "$patch_file" | tr -d ' ')
    
    # Compare
    echo "  Expected SHA1: $expected_sha1"
    echo "  Actual SHA1:   $actual_sha1"
    echo "  Expected size: $expected_size bytes"
    echo "  Actual size:   $actual_size bytes"
    
    if [ "$expected_sha1" = "$actual_sha1" ] && [ "$expected_size" = "$actual_size" ]; then
        echo "  ✓ Verification passed"
        return 0
    else
        echo "  ❌ Verification FAILED"
        if [ "$expected_sha1" != "$actual_sha1" ]; then
            echo "     SHA1 mismatch!"
        fi
        if [ "$expected_size" != "$actual_size" ]; then
            echo "     Size mismatch!"
        fi
        return 1
    fi
}

# Main execution
echo "=== Patch Integrity Verification ==="
echo ""

total=0
passed=0
failed=0

if [ -n "$1" ]; then
    # Verify specific patch
    verify_patch "$1"
    exit $?
else
    # Verify all patches
    for patch_file in "$PATCHES_DIR"/*.patch; do
        if [ -f "$patch_file" ]; then
            total=$((total + 1))
            patch_name=$(basename "$patch_file")
            
            if verify_patch "$patch_name"; then
                passed=$((passed + 1))
            else
                failed=$((failed + 1))
            fi
            echo ""
        fi
    done
    
    echo "=== Summary ==="
    echo "Total patches: $total"
    echo "Passed: $passed ✓"
    echo "Failed: $failed ❌"
    echo ""
    
    if [ $failed -eq 0 ]; then
        echo "✓ All patches verified successfully"
        exit 0
    else
        echo "❌ Some patches failed verification"
        exit 1
    fi
fi
