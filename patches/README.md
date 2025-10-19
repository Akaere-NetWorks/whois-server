# WHOIS Response Patches System - Complete Documentation

This directory contains patch files for automatic text replacement in WHOIS query responses. The patch system provides a flexible way to customize WHOIS responses based on query content and response patterns.

## Table of Contents

1. [Overview](#overview)
2. [File Naming Convention](#file-naming-convention)
3. [Patch File Format](#patch-file-format)
4. [Directives Reference](#directives-reference)
5. [Examples](#examples)
6. [Processing Flow](#processing-flow)
7. [Best Practices](#best-practices)
8. [Debugging](#debugging)
9. [Advanced Usage](#advanced-usage)

---

## Overview

The patch system allows you to:
- **Replace text** in WHOIS responses automatically
- **Apply conditions** based on query input or response content
- **Use case-sensitive or case-insensitive** matching
- **Support regular expressions** for complex patterns
- **Define multiple rules** in a single patch file

### How It Works

```
User Query → WHOIS Processing → Response → Apply Patches → Final Output
                                              ↑
                                         Patch Files
```

1. Server loads all `.patch` files at startup
2. For each query, the response is generated normally
3. Before returning to client, all applicable patches are applied
4. Patches are applied in numerical order (001, 002, 003...)

---

## File Naming Convention

Patch files **must** follow this naming pattern:

```
NNN-description.patch
```

**Components:**
- `NNN` = Three-digit number (001-999)
- `description` = Lowercase description (use hyphens for spaces)
- Extension = `.patch`

**Examples:**
```
001-ruinetwork.patch          ✓ Good
002-example-org.patch         ✓ Good
010-common-branding.patch     ✓ Good
100-network-cleanup.patch     ✓ Good

ruinetwork.patch              ✗ Missing number prefix
1-rui.patch                   ✗ Number should be 3 digits
001_ruinetwork.patch          ✗ Use hyphens, not underscores
001-RuiNetwork.patch          ✗ Description should be lowercase
```

**Processing Order:**
Files are processed in numerical order:
- `001-xxx.patch` first
- `002-xxx.patch` second
- `010-xxx.patch` third
- etc.

---

## Patch File Format

Each patch file contains one or more replacement rules using these directives:

### Basic Structure

```
# Comments start with # and are ignored
# Empty lines are also ignored

--- rule-separator
+++ rule-definition

CONDITION: condition_specification
MATCH_TYPE: matching_method
SEARCH: text_to_find
REPLACE: replacement_text
```

### Section Separators

Multiple rules in one file are separated by:
```
---
```

Each rule starts with:
```
--- rule-separator
+++ rule-definition
```

---

## Directives Reference

### 1. CONDITION (Optional)

Controls **when** the replacement is applied.

#### Syntax Options:

```
CONDITION: ALWAYS
```
- Always applies the replacement (no condition check)
- Use for global replacements

```
CONDITION: QUERY_CONTAINS text
```
- Applies only if the original query contains `text` (case-sensitive)
- Example: `QUERY_CONTAINS AS211575`
  - Matches: `AS211575`, `AS211575-RADB`, `query AS211575`
  - Does NOT match: `as211575`

```
CONDITION: QUERY_CONTAINS_ICASE text
```
- Same as `QUERY_CONTAINS` but case-insensitive
- Example: `QUERY_CONTAINS_ICASE ruinetwork`
  - Matches: `RuiNetwork`, `ruinetwork`, `RUINETWORK`, `AS-RuiNetwork`

```
CONDITION: RESPONSE_CONTAINS text
```
- Applies only if the WHOIS response contains `text`
- Useful for targeted replacements based on response content

```
CONDITION: RESPONSE_CONTAINS_ICASE text
```
- Same as `RESPONSE_CONTAINS` but case-insensitive

**If CONDITION is omitted**, the rule is treated as `CONDITION: ALWAYS`.

### 2. MATCH_TYPE (Required)

Controls **how** to match the search text.

```
MATCH_TYPE: EXACT
```
- Exact string match (case-sensitive)
- Fastest performance
- Use when you know the exact text to replace

```
MATCH_TYPE: ICASE
```
- Case-insensitive match
- Replaces all case variations
- Example: `RuiNetwork`, `ruinetwork`, `RUINETWORK` all match

```
MATCH_TYPE: REGEX
```
- Regular expression match (Rust regex syntax)
- Most powerful but slower
- Supports capture groups: `$1`, `$2`, etc.

### 3. SEARCH (Required)

The text or pattern to find.

**Examples:**
```
SEARCH: RuiNetwork
SEARCH: netname:        RuiNetwork
SEARCH: "text with spaces"
SEARCH: netname:\s+\w+              (regex)
```

**Tips:**
- Use quotes for text with leading/trailing spaces
- Be specific to avoid unintended matches
- Test with various inputs

### 4. REPLACE (Required)

The replacement text.

**Examples:**
```
REPLACE: Ruifeng Enterprise Transit Network
REPLACE: netname:        Ruifeng Enterprise Transit Network
REPLACE: "replacement with spaces"
REPLACE: netname:        $1          (regex with capture group)
```

---

## Examples

### Example 1: Simple Case-Insensitive Replacement

**File: `001-ruinetwork.patch`**

```
# Replace RuiNetwork with full name (case-insensitive)
--- rule-separator
+++ rule-definition

CONDITION: RESPONSE_CONTAINS RuiNetwork
MATCH_TYPE: ICASE
SEARCH: RuiNetwork
REPLACE: Ruifeng Enterprise Transit Network
```

**Effect:**
- Input containing: `RuiNetwork`, `ruinetwork`, `RUINETWORK`
- All become: `Ruifeng Enterprise Transit Network`

### Example 2: Conditional Replacement on Query

**File: `001-ruinetwork.patch` (additional rule)**

```
# Only for AS211575 queries, replace netname field
--- rule-separator
+++ rule-definition

CONDITION: QUERY_CONTAINS_ICASE AS211575
MATCH_TYPE: EXACT
SEARCH: netname:        RuiNetwork
REPLACE: netname:        Ruifeng Enterprise Transit Network
```

**Effect:**
- Only applies when query contains "AS211575" (any case)
- Replaces the specific `netname:` field line

### Example 3: Multiple Rules in One File

**File: `001-ruinetwork.patch` (complete file)**

```
# RuiNetwork Branding Replacements
# This patch handles all RuiNetwork related text replacements

# Rule 1: When query mentions RuiNetwork or AS211575, fix netname field
--- rule-separator
+++ rule-definition

CONDITION: QUERY_CONTAINS_ICASE RuiNetwork AS211575
MATCH_TYPE: EXACT
SEARCH: netname:        RuiNetwork
REPLACE: netname:        Ruifeng Enterprise Transit Network

---

# Rule 2: Replace all remaining occurrences (case-insensitive)
--- rule-separator
+++ rule-definition

CONDITION: RESPONSE_CONTAINS RuiNetwork
MATCH_TYPE: ICASE
SEARCH: RuiNetwork
REPLACE: Ruifeng Enterprise Transit Network
```

### Example 4: Regex Replacement

**File: `002-descr-cleanup.patch`**

```
# Clean up description fields
--- rule-separator
+++ rule-definition

CONDITION: ALWAYS
MATCH_TYPE: REGEX
SEARCH: descr:\s+RuiNetwork.*
REPLACE: descr:          Ruifeng Enterprise Transit Network - Premium Services
```

---

## Processing Flow

### Startup Phase

```
1. Server starts
2. Scans ./patches/ directory
3. Loads all *.patch files in numerical order
4. Parses each file and stores rules
5. Logs: "Loaded N patch files with M rules total"
```

### Query Phase

```
For each WHOIS query:
  1. Process query normally → Generate response
  2. For each patch file (001, 002, 003...):
      a. For each rule in the file:
          i.   Check CONDITION (if any)
          ii.  If condition met:
               - Apply SEARCH → REPLACE based on MATCH_TYPE
          iii. Continue to next rule
  3. Return final patched response to client
```

### Example Flow

```
Query: "AS211575"
↓
Normal Processing: Generate response with "netname: RuiNetwork"
↓
Apply 001-ruinetwork.patch:
  Rule 1: Check QUERY_CONTAINS_ICASE AS211575 → ✓ Match
          Replace "netname:        RuiNetwork" → "netname:        Ruifeng Enterprise Transit Network"
  Rule 2: Check RESPONSE_CONTAINS RuiNetwork → ✓ Match
          Replace "RuiNetwork" → "Ruifeng Enterprise Transit Network" (case-insensitive)
↓
Return patched response to client
```

---

## Best Practices

### 1. Naming and Organization

✓ **DO:**
- Use descriptive filenames: `001-ruinetwork.patch`
- Add comments explaining each rule
- Group related replacements in one file

✗ **DON'T:**
- Use generic names: `patch.patch`
- Leave files uncommented
- Split related rules across many files unnecessarily

### 2. Condition Usage

✓ **DO:**
- Use specific conditions to limit scope
- Combine `QUERY_CONTAINS` with exact field replacements
- Use `ALWAYS` sparingly

✗ **DON'T:**
- Make everything `CONDITION: ALWAYS` (impacts performance)
- Forget to test edge cases
- Create conflicting conditions

### 3. Match Type Selection

**Use EXACT when:**
- You know the exact text to replace
- Performance is critical
- Text is well-defined and won't vary

**Use ICASE when:**
- Case variations exist (RuiNetwork, ruinetwork)
- User input might vary in case
- Branding consistency is needed

**Use REGEX when:**
- Pattern matching is required
- Need to capture and reuse parts of text
- Complex replacements needed

### 4. Testing

Before deploying a patch:

```bash
# 1. Create test patch file
vim patches/999-test.patch

# 2. Restart server
./whois-server --debug

# 3. Test with actual query
whois -h localhost AS211575

# 4. Verify in debug logs
# Look for: "Applying patch rule..." messages

# 5. Remove test file
rm patches/999-test.patch
```

### 5. Performance Optimization

**Order rules efficiently:**
```
# Fast (specific condition, exact match)
CONDITION: QUERY_CONTAINS AS211575
MATCH_TYPE: EXACT
SEARCH: netname:        RuiNetwork
REPLACE: netname:        Ruifeng Enterprise

# Slower (broad condition, regex)
CONDITION: ALWAYS
MATCH_TYPE: REGEX
SEARCH: \w+Network
REPLACE: Enterprise Network
```

**Guidelines:**
- Put specific conditions before `ALWAYS`
- Use `EXACT` or `ICASE` instead of `REGEX` when possible
- Limit the number of `ALWAYS` rules
- Test with `--debug` to see performance impact

---

## Debugging

### Enable Debug Logging

```bash
./whois-server --debug
```

### Debug Output Example

```
[INFO] Loading response patches from ./patches directory...
[DEBUG] Found patch file: 001-ruinetwork.patch
[DEBUG] Parsing patch file: 001-ruinetwork.patch
[DEBUG] Loaded rule: QUERY_CONTAINS_ICASE RuiNetwork
[INFO] Loaded 1 patch files with 2 rules total
...
[DEBUG] Processing query: AS211575
[DEBUG] Generated response (500 bytes)
[DEBUG] Applying patches to response...
[DEBUG] Evaluating rule from 001-ruinetwork.patch
[DEBUG] Condition met: QUERY_CONTAINS_ICASE AS211575
[DEBUG] Applying EXACT replacement
[DEBUG] Replaced 1 occurrence(s)
[DEBUG] Final response (520 bytes)
```

### Common Issues

**Issue: Patch not applying**

Checklist:
- [ ] File named correctly? (NNN-name.patch)
- [ ] Server restarted after adding patch?
- [ ] CONDITION is met? (check debug logs)
- [ ] SEARCH text matches exactly? (if using EXACT)
- [ ] Correct directory? (./patches/)

**Issue: Wrong text replaced**

Solutions:
- Make SEARCH more specific
- Add CONDITION to limit scope
- Check rule order (earlier rules might interfere)
- Use EXACT instead of ICASE/REGEX if possible

**Issue: Performance degradation**

Solutions:
- Reduce number of REGEX rules
- Add CONDITIONS to limit when rules apply
- Use EXACT instead of ICASE when possible
- Profile with `--debug` to find slow rules

---

## Advanced Usage

### Combining Multiple Conditions

While not directly supported, you can achieve AND logic:

```
# Step 1: Mark text if query condition is met
CONDITION: QUERY_CONTAINS AS211575
MATCH_TYPE: EXACT
SEARCH: RuiNetwork
REPLACE: __MARKED__RuiNetwork__

---

# Step 2: Final replacement only if both conditions met
CONDITION: RESPONSE_CONTAINS __MARKED__
MATCH_TYPE: EXACT
SEARCH: __MARKED__RuiNetwork__
REPLACE: Ruifeng Enterprise Transit Network
```

### Preserving Exact Formatting

Use quotes to preserve spaces:

```
SEARCH: "netname:        RuiNetwork"
REPLACE: "netname:        Ruifeng Enterprise Transit Network"
```

### Regex Capture Groups

```
MATCH_TYPE: REGEX
SEARCH: (netname|descr):\s+(\w+)
REPLACE: $1:          Ruifeng $2 Network
```

Result:
- `netname:        RuiNetwork` → `netname:          Ruifeng RuiNetwork Network`

### Case Transformation (via regex)

```
MATCH_TYPE: REGEX
SEARCH: (?i)(ruinetwork)
REPLACE: Ruifeng Enterprise
```

The `(?i)` makes the pattern case-insensitive.

---

## Security and Permissions

### File Permissions

Recommended settings:
```bash
chmod 755 patches/           # Directory: rwxr-xr-x
chmod 644 patches/*.patch    # Files: rw-r--r--
```

### Validation

- Patch files are only read at server startup by admin
- No user input is used in patch file paths
- All regex patterns are validated at load time
- Invalid patches are logged and skipped

### Resource Limits

- Max patch file size: 1 MB
- Max regex complexity: Limited by Rust regex engine
- No limit on number of rules (within reason)

---

## Migration from Old Format

If you have old-style patch files:

**Old format:**
```
QUERY_CONTAINS: RuiNetwork
REPLACE: oldtext
WITH: newtext
```

**New format:**
```
--- rule-separator
+++ rule-definition

CONDITION: QUERY_CONTAINS RuiNetwork
MATCH_TYPE: EXACT
SEARCH: oldtext
REPLACE: newtext
```

**Migration script:**
```bash
# Backup old files
cp patches/file.patch patches/file.patch.bak

# Manually convert to new format
# (automatic conversion not yet available)
```

---

## Version History

- **v1.0** (2025-01-19)
  - Initial implementation
  - Support for CONDITION, MATCH_TYPE, SEARCH, REPLACE
  - Case-sensitive and case-insensitive matching
  - Regular expression support
  - Query and response conditions

---

## Support

For issues or questions:

1. Check this documentation
2. Review debug logs with `--debug` flag
3. Test with simplified patch files
4. Report issues on GitHub with example patch file

---

## Quick Reference

### Minimal Patch File

```
--- rule-separator
+++ rule-definition

MATCH_TYPE: EXACT
SEARCH: oldtext
REPLACE: newtext
```

### Full-Featured Patch File

```
# Comment describing the patch
--- rule-separator
+++ rule-definition

CONDITION: QUERY_CONTAINS_ICASE keyword
MATCH_TYPE: ICASE
SEARCH: text to find
REPLACE: replacement text
```

### Common Patterns

```
# Replace specific field
MATCH_TYPE: EXACT
SEARCH: netname:        OldName
REPLACE: netname:        NewName

# Case-insensitive global replace
MATCH_TYPE: ICASE
SEARCH: oldterm
REPLACE: newterm

# Regex pattern with capture
MATCH_TYPE: REGEX
SEARCH: (field):\s+(.+)
REPLACE: $1:          New $2
```
