# WHOIS Response Patches System - Complete Documentation

This directory contains patch files for automatic text replacement in WHOIS query responses. The patch system provides a flexible, remotely-managed way to customize WHOIS responses based on query content and response patterns.

## Table of Contents

1. [Overview](#overview)
2. [Remote Update System](#remote-update-system)
3. [File Naming Convention](#file-naming-convention)
4. [Patch File Format](#patch-file-format)
5. [Directives Reference](#directives-reference)
6. [Examples](#examples)
7. [Processing Flow](#processing-flow)
8. [Best Practices](#best-practices)
9. [Debugging](#debugging)
10. [Advanced Usage](#advanced-usage)
11. [Patch Metadata (patches.json)](#patch-metadata-patchesjson)
12. [Online Updates](#online-updates)

---

## Overview

The patch system allows you to:
- **Replace text** in WHOIS responses automatically
- **Apply conditions** based on query input or response content
- **Use context-aware rules** to avoid unwanted replacements
- **Update patches remotely** from GitHub repository
- **Verify integrity** with SHA1 checksums
- **Store in LMDB** for fast loading and persistence
- **Define multiple rules** in a single patch file

### Architecture (Remote + LMDB)

```
GitHub Repository → UPDATE-PATCH Command → Download & Verify → LMDB Storage
                         (SHA1 Check)                              ↓
User Query → WHOIS Processing → Response → Apply Patches → Final Output
                                              ↑
                                      Load from LMDB
```

**Key Features:**
- ✅ Patches loaded from LMDB cache (not local files)
- ✅ Updates triggered by `whois UPDATE-PATCH` command
- ✅ SHA1 checksum verification for security
- ✅ Automatic download from GitHub repository
- ✅ No patches loaded on server startup by default
- ✅ Persistent storage survives server restarts

---

## Remote Update System

### Updating Patches

To update patches from the remote repository:

```bash
# Query the UPDATE-PATCH command
whois -h whois.akae.re UPDATE-PATCH

# Or using netcat
echo "UPDATE-PATCH" | nc whois.akae.re 43
```

### Update Process

1. **Download** `patches.json` from GitHub
2. **Parse** metadata (patch names, URLs, SHA1 checksums)
3. **Download** each enabled patch file
4. **Verify** SHA1 checksum
5. **Store** in LMDB cache at `./cache/patches_cache/`
6. **Reload** patches into memory

### Output Format

The UPDATE-PATCH command returns a WHOIS-formatted report:

```
% Patch Update Report
% Downloaded from: https://raw.githubusercontent.com/Akaere-NetWorks/whois-server/refs/heads/main/patches/patches.json
% Last Updated: 2025-01-19T10:30:00Z
% Format Version: 1.0
%
patch:           001-ruinetwork.patch
description:     RuiNetwork branding for AS211575
url:             https://raw.githubusercontent.com/Akaere-NetWorks/whois-server/refs/heads/main/patches/001-ruinetwork.patch
sha1-expected:   a1b2c3d4e5f6...
sha1-actual:     a1b2c3d4e5f6...
size-expected:   1234 bytes
priority:        10
modified:        2025-01-19T08:00:00Z
status:          ✓ VERIFIED

% Summary
% Total patches: 1
% Successful: 1
% Failed: 0
%
% Run 'whois help' for more information
```

### Repository Structure

```
https://github.com/Akaere-NetWorks/whois-server/
└── patches/
    ├── patches.json           # Metadata index
    ├── 001-ruinetwork.patch   # Individual patches
    ├── 002-example.patch
    └── README.md              # This file
```

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

### 5. EXCLUDE (Optional) - Blacklist Feature

**NEW:** Exclude specific lines from replacement, even if they match the search pattern.

#### Syntax:
```
EXCLUDE: pattern
```

The `EXCLUDE` directive creates a **blacklist** - any line containing the specified pattern will be **skipped** during replacement, preserving the original text.

**Use Cases:**
- Preserve maintainer references (`mnt-by:` fields)
- Keep original identifiers intact
- Protect specific metadata from replacement

**Examples:**

```
# Exclude mnt-by field from RuiNetwork replacement
# EXCLUDE: mnt-by:

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-RuiNetwork
+Ruifeng Enterprise Transit Network
```

With this exclude pattern:
- `descr: RuiNetwork` → `descr: Ruifeng Enterprise Transit Network` ✓ Replaced
- `as-name: RuiNetwork` → `as-name: Ruifeng Enterprise Transit Network` ✓ Replaced
- `mnt-by: RUINETWORK-MNT` → `mnt-by: RUINETWORK-MNT` ✗ **Preserved (not replaced)**

**Multiple Exclusions:**

```
# EXCLUDE: mnt-by:
# EXCLUDE: remarks:
# EXCLUDE: admin-c:
```

All lines containing `mnt-by:`, `remarks:`, or `admin-c:` will be excluded from replacement.

**How It Works:**
1. Patch system identifies text to replace
2. Before applying replacement, checks if the line contains any exclude pattern
3. If match found → **skip replacement**, keep original
4. If no match → apply replacement as normal

### 6. Context Rules (Optional) - Advanced Line Context Control

**NEW:** Control replacements based on surrounding lines context. These rules allow you to apply replacements conditionally based on what appears in nearby lines.

#### 6.1 SKIP_BEFORE - Skip if pattern found above

Skip replacement if a specific pattern is found in N lines **before** the current line.

**Syntax:**
```
# SKIP_BEFORE: pattern, lines
```

- `pattern`: Text to search for (case-sensitive)
- `lines`: Number of lines to look back (1-100)

**Example:**
```
# Skip source: replacement if as-block: found within 7 lines before
# This prevents replacing IANA registry blocks
# SKIP_BEFORE: as-block:, 7

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-^source:
+source:          MoeDove
```

**Behavior:**
```
as-block:        AS208189 - AS215128    ← Found as-block: here
descr:           RIPE NCC ASN block
remarks:         These AS Numbers are assigned...
mnt-by:          RIPE-NCC-HM-MNT
created:         2025-03-25T09:02:29Z
last-modified:   2025-03-25T09:02:29Z
source:          RIPE                   ← Within 7 lines, SKIP replacement
```

vs.

```
aut-num:         AS211575               ← No as-block: in this section
remarks:         Website: https://...
descr:           Ruifeng Enterprise...
as-name:         Ruifeng Enterprise...
org:             ORG-RF108-RIPE
...
source:          RIPE                   ← APPLY replacement → MoeDove
```

#### 6.2 SKIP_AFTER - Skip if pattern found below

Skip replacement if a specific pattern is found in N lines **after** the current line.

**Syntax:**
```
# SKIP_AFTER: pattern, lines
```

**Example:**
```
# Skip replacement if "% Filtered" appears within 3 lines after
# SKIP_AFTER: % Filtered, 3
```

#### 6.3 ONLY_BEFORE - Only replace if pattern found above

Only apply replacement if a specific pattern is found in N lines **before** the current line.

**Syntax:**
```
# ONLY_BEFORE: pattern, lines
```

**Example:**
```
# Only replace source: if aut-num: appears within 20 lines above
# This ensures we only patch user objects, not registry data
# ONLY_BEFORE: aut-num:, 20

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-^source:
+source:          MoeDove
```

#### 6.4 ONLY_AFTER - Only replace if pattern found below

Only apply replacement if a specific pattern is found in N lines **after** the current line.

**Syntax:**
```
# ONLY_AFTER: pattern, lines
```

**Example:**
```
# Only replace if "created:" field appears within 5 lines after
# ONLY_AFTER: created:, 5
```

#### Multiple Context Rules

You can combine multiple context rules. **ALL** rules must pass for the replacement to occur:

```
# SKIP_BEFORE: as-block:, 7
# ONLY_BEFORE: aut-num:, 20
# SKIP_AFTER: % End of query, 5
```

**Logic:**
1. Check SKIP_BEFORE → if matches, skip replacement
2. Check SKIP_AFTER → if matches, skip replacement
3. Check ONLY_BEFORE → if doesn't match, skip replacement
4. Check ONLY_AFTER → if doesn't match, skip replacement
5. If all checks pass → apply replacement

#### Context Rules Use Cases

**Use Case 1: Protect Registry Data**
```
# Don't modify source: in as-block, inetnum, route objects
# SKIP_BEFORE: as-block:, 10
# SKIP_BEFORE: inetnum:, 10
# SKIP_BEFORE: route:, 10
```

**Use Case 2: Target Specific Objects Only**
```
# Only modify source: in aut-num objects (ASN records)
# ONLY_BEFORE: aut-num:, 20
```

**Use Case 3: Avoid Filtered Sections**
```
# Skip replacement in filtered output sections
# SKIP_AFTER: # Filtered, 2
```

**Use Case 4: Complex Logic**
```
# Replace source only in person/org objects, but not in filtered sections
# ONLY_BEFORE: person:|organisation:, 15
# SKIP_AFTER: # Filtered, 3
```

#### Special Pattern: Line Start Matching (^)

For line-start matching patterns (e.g., `^source:`), ANSI color codes are automatically stripped before checking context rules, ensuring accurate matching in colored output.

```
# This works with both colored and plain text output
# SKIP_BEFORE: as-block:, 7

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-^source:
+source:          MoeDove
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

### Example 5: Using EXCLUDE Directive (Blacklist)

**File: `001-ruinetwork.patch`**

```
# RuiNetwork branding replacements
# Exclude mnt-by field to preserve original RUINETWORK-MNT reference
# EXCLUDE: mnt-by:

# QUERY_CONTAINS: AS211575
# QUERY_CONTAINS: RuiNetwork

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-as-name:         RuiNetwork
+as-name:         Ruifeng Enterprise Transit Network

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-descr:           RuiNetwork
+descr:           Ruifeng Enterprise Transit Network

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-RuiNetwork
+Ruifeng Enterprise Transit Network
```

**Before applying patch:**
```
as-name:         RuiNetwork
descr:           RuiNetwork
mnt-by:          RUINETWORK-MNT
remarks:         Managed by RuiNetwork Team
```

**After applying patch with EXCLUDE:**
```
as-name:         Ruifeng Enterprise Transit Network
descr:           Ruifeng Enterprise Transit Network
mnt-by:          RUINETWORK-MNT            ← Preserved!
remarks:         Managed by Ruifeng Enterprise Transit Network Team
```

The `mnt-by:` line is **excluded** from replacement, preserving the original `RUINETWORK-MNT` identifier.

### Example 6: Using Context Rules - SKIP_BEFORE

**File: `001-ruinetwork.patch`**

```
# RuiNetwork branding replacements with smart source: replacement
# Exclude mnt-by field to preserve original RUINETWORK-MNT
# EXCLUDE: mnt-by:

# Context rules for source: replacement
# Skip source: replacement if as-block: found within 7 lines before
# This prevents replacing IANA registry blocks
# SKIP_BEFORE: as-block:, 7

# QUERY_CONTAINS: AS211575
# QUERY_CONTAINS: RuiNetwork

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-as-name:         RuiNetwork
+as-name:         Ruifeng Enterprise Transit Network (RETN)

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-descr:           RuiNetwork
+descr:           Ruifeng Enterprise Transit Network (RETN)

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-^source:
+source:          MoeDove
```

**Input WHOIS Response:**
```
% Information related to 'AS208189 - AS215128'

as-block:        AS208189 - AS215128
descr:           RIPE NCC ASN block
remarks:         These AS Numbers are assigned...
mnt-by:          RIPE-NCC-HM-MNT
created:         2025-03-25T09:02:29Z
last-modified:   2025-03-25T09:02:29Z
source:          RIPE                    ← as-block: is 6 lines above

% Information related to 'AS211575'

aut-num:         AS211575
remarks:         Website: https://net.fengrui.link
descr:           Ruifeng Enterprise Transit Network (RETN)
as-name:         Ruifeng Enterprise Transit Network (RETN)
org:             ORG-RF108-RIPE
...
source:          RIPE                    ← No as-block: in 7 lines above
```

**Output After Patch:**
```
% Information related to 'AS208189 - AS215128'

as-block:        AS208189 - AS215128
descr:           RIPE NCC ASN block
remarks:         These AS Numbers are assigned...
mnt-by:          RIPE-NCC-HM-MNT
created:         2025-03-25T09:02:29Z
last-modified:   2025-03-25T09:02:29Z
source:          RIPE                    ← SKIPPED (as-block: nearby)

% Information related to 'AS211575'

aut-num:         AS211575
remarks:         Website: https://net.fengrui.link
descr:           Ruifeng Enterprise Transit Network (RETN)
as-name:         Ruifeng Enterprise Transit Network (RETN)
org:             ORG-RF108-RIPE
...
source:          MoeDove                 ← REPLACED (no as-block: nearby)
```

**Explanation:**
- The first `source:` field is within 7 lines of `as-block:` → **skipped**
- The second `source:` field has no `as-block:` in the previous 7 lines → **replaced**
- This prevents accidentally modifying IANA registry data while still patching user objects

### Example 7: Using Multiple Context Rules

**File: `002-advanced-patching.patch`**

```
# Advanced patching with multiple context rules
# Only replace source: in aut-num/organisation/person objects
# But skip if in filtered sections

# ONLY_BEFORE: aut-num:|organisation:|person:, 20
# SKIP_AFTER: # Filtered, 3
# QUERY_CONTAINS: AS211575

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-^source:
+source:          CustomDB
```

**Logic Flow:**
1. Find `source:` line
2. Check: Is `aut-num:` OR `organisation:` OR `person:` within 20 lines above? 
   - If NO → skip replacement
3. Check: Is `# Filtered` within 3 lines below?
   - If YES → skip replacement
4. If both checks pass → apply replacement
descr:           Ruifeng Enterprise Transit Network
mnt-by:          RUINETWORK-MNT            ← Preserved!
remarks:         Managed by Ruifeng Enterprise Transit Network Team
```

The `mnt-by:` line is **excluded** from replacement, preserving the original `RUINETWORK-MNT` identifier.

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
- Flexible matching is essential

### 4. EXCLUDE Usage Best Practices

✓ **DO:**
- Document why each exclusion is needed
- Use specific patterns (`mnt-by:` not just `mnt`)
- Test that exclusions work as intended

✗ **DON'T:**
- Over-exclude (blocks too many legitimate replacements)
- Use overly broad patterns
- Forget to test edge cases

### 5. Context Rules Best Practices

✓ **DO:**
- Use SKIP_BEFORE/AFTER to protect registry data
- Use ONLY_BEFORE/AFTER to target specific object types
- Set reasonable line limits (7-20 lines typically sufficient)
- Document the purpose of each context rule
- Test with real WHOIS responses

✗ **DON'T:**
- Set line limits too high (>50) - impacts performance
- Set line limits too low (<5) - may miss context
- Combine too many context rules (can be confusing)
- Forget that WHOIS blocks are separated by empty lines

**Context Rules Guidelines:**
- For WHOIS object identification: 10-20 lines is usually enough
- For short-range protection: 3-7 lines
- For filtering sections: 5-10 lines
- Test with actual WHOIS output to verify line counts

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

## Quick Reference Card

### All Available Directives

| Directive | Syntax | Purpose | Example |
|-----------|--------|---------|---------|
| **QUERY_CONTAINS** | `# QUERY_CONTAINS: text` | Condition: query contains text | `# QUERY_CONTAINS: AS211575` |
| **RESPONSE_CONTAINS** | `# RESPONSE_CONTAINS: text` | Condition: response contains text | `# RESPONSE_CONTAINS: RuiNetwork` |
| **EXCLUDE** | `# EXCLUDE: pattern` | Blacklist: skip lines with pattern | `# EXCLUDE: mnt-by:` |
| **SKIP_BEFORE** | `# SKIP_BEFORE: pattern, N` | Skip if pattern in N lines above | `# SKIP_BEFORE: as-block:, 7` |
| **SKIP_AFTER** | `# SKIP_AFTER: pattern, N` | Skip if pattern in N lines below | `# SKIP_AFTER: # Filtered, 3` |
| **ONLY_BEFORE** | `# ONLY_BEFORE: pattern, N` | Only if pattern in N lines above | `# ONLY_BEFORE: aut-num:, 20` |
| **ONLY_AFTER** | `# ONLY_AFTER: pattern, N` | Only if pattern in N lines below | `# ONLY_AFTER: created:, 5` |

### Directive Categories

**Conditions** (when to apply patch):
- `QUERY_CONTAINS` - based on user's query
- `RESPONSE_CONTAINS` - based on WHOIS response

**Exclusions** (what to protect):
- `EXCLUDE` - blacklist specific line patterns

**Context Rules** (surrounding lines logic):
- `SKIP_BEFORE` / `SKIP_AFTER` - negative conditions (don't replace if...)
- `ONLY_BEFORE` / `ONLY_AFTER` - positive conditions (only replace if...)

### Special Syntax

| Syntax | Meaning | Example |
|--------|---------|---------|
| `^source:` | Match line starting with "source:" | Replace entire line |
| `\|` in patterns | OR operator in ONLY/SKIP rules | `aut-num:\|organisation:` |
| Color-aware | ANSI codes auto-stripped for matching | Works with colored output |

### Common Patterns

**Protect registry data:**
```
# SKIP_BEFORE: as-block:, 7
# SKIP_BEFORE: inetnum:, 7
# SKIP_BEFORE: route:, 7
```

**Target user objects only:**
```
# ONLY_BEFORE: aut-num:|organisation:|person:, 20
```

**Preserve maintainer info:**
```
# EXCLUDE: mnt-by:
# EXCLUDE: mnt-ref:
```

**Replace entire field:**
```
--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-^source:
+source:          MoeDove
```

### Execution Order

1. Load all `.patch` files (001, 002, 003, ...)
2. For each query response:
   - Check CONDITIONS → skip if not matched
   - Check EXCLUDE patterns → skip matching lines
   - Check CONTEXT RULES → skip if rules fail
   - Apply replacement
3. Return patched response

### Troubleshooting

**Problem: Replacement not working**
- ✓ Check condition matches (QUERY_CONTAINS/RESPONSE_CONTAINS)
- ✓ Verify no EXCLUDE pattern matches the line
- ✓ Check context rules (SKIP_BEFORE/AFTER, ONLY_BEFORE/AFTER)
- ✓ Enable `--debug` to see detailed logs

**Problem: Too many replacements**
- ✓ Add EXCLUDE patterns for protected lines
- ✓ Add SKIP_BEFORE/AFTER for specific blocks
- ✓ Use more specific SEARCH patterns

**Problem: Context rule not working**
- ✓ Check line count is sufficient (count actual lines in output)
- ✓ Verify pattern exactly matches (case-sensitive)
- ✓ Remember WHOIS blocks are separated by empty lines
- ✓ Check debug logs for context rule evaluation

---

## Complete Example File

**File: `001-ruinetwork.patch`**

```
# RuiNetwork branding replacements
# This patch handles automatic text replacement for RuiNetwork/AS211575 queries
# Ensures consistent branding: RuiNetwork → Ruifeng Enterprise Transit Network (RETN)

# Exclude mnt-by field from replacement to preserve original RUINETWORK-MNT
# EXCLUDE: mnt-by:

# Context rules for source: replacement
# Skip source: replacement if as-block: found within 7 lines before
# This prevents replacing IANA registry blocks
# SKIP_BEFORE: as-block:, 7

# Rule 1: When query contains "RuiNetwork" or "AS211575" (case-insensitive)
# Replace netname, as-name, descr and source fields
# QUERY_CONTAINS: RuiNetwork
# QUERY_CONTAINS: AS211575
# QUERY_CONTAINS: ruinetwork
# QUERY_CONTAINS: as211575

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-as-name:         RuiNetwork
+as-name:         Ruifeng Enterprise Transit Network (RETN)

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-descr:           RuiNetwork
+descr:           Ruifeng Enterprise Transit Network (RETN)

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-^source:
+source:          MoeDove

# Rule 2: When response contains "RuiNetwork" in any form
# Replace all occurrences and update source field
# RESPONSE_CONTAINS: RuiNetwork
--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-as-name:         RuiNetwork
+as-name:         Ruifeng Enterprise Transit Network (RETN)

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-descr:           RuiNetwork
+descr:           Ruifeng Enterprise Transit Network (RETN)

--- original_response
+++ patched_response
@@ -1,1 +1,1 @@
-RuiNetwork
+Ruifeng Enterprise Transit Network (RETN)
```

---

## Patch Metadata (patches.json)

### Overview

The `patches.json` file contains metadata about all available patch files, including URLs, checksums, and versioning information. This file is designed to support:

- **Online updates**: Download patches from remote repositories
- **Integrity verification**: Verify patch files haven't been tampered with
- **Version management**: Track patch versions and dependencies
- **Automated deployment**: Enable automated patch distribution

### File Structure

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "version": "1.0.0",
  "description": "WHOIS Server Patch Metadata",
  "repository": "https://github.com/Akaere-NetWorks/whois-server",
  "last_updated": "2025-10-19T18:03:45+08:00",
  "patches": [
    {
      "name": "001-ruinetwork.patch",
      "description": "RuiNetwork branding replacements",
      "url": "https://raw.githubusercontent.com/.../001-ruinetwork.patch",
      "sha1": "4b6d69ddc59b8f45e54aae67e3d6a45f26f26ee4",
      "size": 2700,
      "enabled": true,
      "priority": 1,
      "modified": "2025-10-19T17:55:17+08:00"
    }
  ],
  "metadata": {
    "format_version": "1.0",
    "checksum_algorithm": "SHA1",
    "update_url": "https://raw.githubusercontent.com/.../patches.json",
    "documentation": "https://github.com/.../README.md"
  }
}
```

### Field Descriptions

#### Root Fields

| Field | Type | Description |
|-------|------|-------------|
| `$schema` | String | JSON Schema URI for validation |
| `version` | String | Metadata format version (semver) |
| `description` | String | Human-readable description |
| `repository` | String | Source repository URL |
| `last_updated` | String | ISO 8601 timestamp of last update |
| `patches` | Array | List of patch metadata objects |
| `metadata` | Object | System metadata |

#### Patch Object Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | String | ✓ | Patch filename (must match actual file) |
| `description` | String | ✓ | Brief description of patch purpose |
| `url` | String | ✓ | Direct download URL (raw GitHub content) |
| `sha1` | String | ✓ | SHA1 checksum for integrity verification |
| `size` | Integer | ✓ | File size in bytes |
| `enabled` | Boolean | ✓ | Whether patch is active (default: true) |
| `priority` | Integer | ✓ | Execution order (1 = first) |
| `modified` | String | ✓ | ISO 8601 timestamp of last modification |
| `version` | String | ✗ | Patch version (semver) |
| `author` | String | ✗ | Author/maintainer name |
| `tags` | Array | ✗ | Tags for categorization |
| `dependencies` | Array | ✗ | Required patches (by name) |

#### Metadata Object Fields

| Field | Type | Description |
|-------|------|-------------|
| `format_version` | String | patches.json format version |
| `checksum_algorithm` | String | Hash algorithm used (SHA1, SHA256, etc.) |
| `update_url` | String | URL to fetch latest patches.json |
| `documentation` | String | URL to documentation |

### Updating patches.json

#### Automatic Update Script

Use the provided script to automatically regenerate `patches.json`:

```bash
cd patches/
./update-patches-json.sh
```

The script will:
1. Scan for all `*.patch` files
2. Calculate SHA1 checksums
3. Extract file sizes and modification times
4. Generate updated `patches.json`

#### Manual Update

To manually add a new patch entry:

1. **Calculate SHA1 checksum:**
   ```bash
   sha1sum 002-new-patch.patch
   ```

2. **Get file size:**
   ```bash
   wc -c < 002-new-patch.patch
   ```

3. **Add to patches.json:**
   ```json
   {
     "name": "002-new-patch.patch",
     "description": "Description of what this patch does",
     "url": "https://raw.githubusercontent.com/Akaere-NetWorks/whois-server/refs/heads/main/patches/002-new-patch.patch",
     "sha1": "abc123...",
     "size": 1234,
     "enabled": true,
     "priority": 2,
     "modified": "2025-10-19T18:00:00+08:00"
   }
   ```

4. **Update `last_updated` field:**
   ```bash
   date -Iseconds
   ```

### Integrity Verification

To verify a patch file's integrity:

```bash
# Calculate current checksum
sha1sum 001-ruinetwork.patch

# Compare with patches.json
grep -A 5 "001-ruinetwork.patch" patches.json | grep sha1
```

If checksums don't match, the file may have been modified or corrupted.

---

## Online Updates

### Future Implementation

The `patches.json` file is designed to support future online update features:

#### Implementation Status

✅ **IMPLEMENTED** - The online update system is fully functional!

1. **Remote Update Command**
   - Command: `whois -h whois.akae.re UPDATE-PATCH`
   - Downloads `patches.json` from GitHub
   - Fetches all enabled patches
   - Verifies SHA1 checksums
   - Stores in LMDB cache

2. **LMDB Storage**
   - Patches stored at `./cache/patches_cache/`
   - Fast loading on server startup
   - Persistent across restarts
   - No need for local patch files

3. **Integrity Verification**
   - SHA1 checksum validation
   - Failed checksums reported in output
   - Only verified patches are stored

4. **WHOIS-Formatted Output**
   - Detailed update report
   - Per-patch verification status
   - Summary statistics

### Update Workflow (Implemented)

```
1. User queries: whois -h whois.akae.re UPDATE-PATCH
2. Server fetches patches.json from GitHub
3. For each enabled patch:
   a. Download patch content from URL
   b. Calculate SHA1 checksum
   c. Compare with expected checksum
   d. If match: store in LMDB
   e. If mismatch: report error
4. Return detailed WHOIS report
5. Patches immediately available for queries
```

### Update Command Usage

**Via WHOIS client:**
```bash
whois -h whois.akae.re UPDATE-PATCH
```

**Via netcat:**
```bash
echo "UPDATE-PATCH" | nc whois.akae.re 43
```

**Via telnet:**
```bash
telnet whois.akae.re 43
> UPDATE-PATCH
```

### Example Output

```
% Patch Update Report
% Downloaded from: https://raw.githubusercontent.com/Akaere-NetWorks/whois-server/refs/heads/main/patches/patches.json
% Last Updated: 2025-01-19T10:30:00Z
% Format Version: 1.0
%

patch:           001-ruinetwork.patch
description:     RuiNetwork branding for AS211575
url:             https://raw.githubusercontent.com/Akaere-NetWorks/whois-server/refs/heads/main/patches/001-ruinetwork.patch
sha1-expected:   a1b2c3d4e5f67890abcdef1234567890abcdef12
sha1-actual:     a1b2c3d4e5f67890abcdef1234567890abcdef12
size-expected:   1234 bytes
priority:        10
modified:        2025-01-19T08:00:00Z
status:          ✓ VERIFIED

% Summary
% Total patches: 1
% Successful: 1
% Failed: 0
%
% Run 'whois help' for more information
```

### No Manual Updates Needed!

The old manual update process is **no longer required**. Simply use:

```bash
whois -h whois.akae.re UPDATE-PATCH
```

All patches are automatically:
- Downloaded from GitHub
- Verified with SHA1 checksums
- Stored in LMDB
- Ready for immediate use

### Security Considerations

✅ **Implemented Security Features:**

1. **SHA1 Checksum Verification**
   - Every patch is verified before storage
   - Mismatched checksums are rejected
   - Verification status shown in update report

2. **HTTPS-Only Downloads**
   - GitHub raw content uses HTTPS
   - TLS certificate validation via rustls
   - Man-in-the-middle protection

3. **Read-Only GitHub Source**
   - Patches fetched from public GitHub repo
   - No write access from server
   - Immutable source of truth

4. **LMDB Storage Security**
   - Local file system permissions apply
   - Cache directory: `./cache/patches_cache/`
   - Standard UNIX file permissions

5. **No Automatic Updates**
   - Updates only via explicit `UPDATE-PATCH` command
   - Manual trigger required
   - Admin has full control

**Best Practices:**

- ✅ Review `patches.json` changes before updating
- ✅ Monitor update logs for failures
- ✅ Keep GitHub repository access restricted
- ✅ Use `update-patches-json.sh` to maintain metadata
- ✅ Test patches in development environment first

### Repository Structure

For online updates to work, maintain this GitHub structure:

```
https://github.com/Akaere-NetWorks/whois-server/
└── patches/
    ├── patches.json           # Metadata index (auto-generated)
    ├── 001-ruinetwork.patch   # Patch files
    ├── 002-example.patch
    ├── README.md              # This documentation
    ├── update-patches-json.sh # Metadata generator
    └── verify-patches.sh      # Verification script
```

**Update workflow for maintainers:**

1. Create/modify `.patch` files in `patches/` directory
2. Run `./update-patches-json.sh` to regenerate metadata
3. Verify with `./verify-patches.sh`
4. Commit both patches and `patches.json`
5. Push to GitHub `main` branch
6. Users can now update via `UPDATE-PATCH` command

### LMDB Storage Details

**Cache Location:** `./cache/patches_cache/`

**Storage Keys:**
- `patch:001-ruinetwork.patch` - Patch content
- `meta:001-ruinetwork.patch` - Patch metadata (JSON)

**Storage Format:**
- Keys: UTF-8 strings
- Values: UTF-8 strings (patch content or JSON)
- Database: Single LMDB environment
- Map size: 1GB maximum

**Loading Priority:**
1. Server checks LMDB cache on startup
2. If cache empty: patches not loaded (normal)
3. To populate: run `UPDATE-PATCH` command
4. Cache persists across server restarts

whois-server/
├── patches/
│   ├── patches.json          ← Metadata index
│   ├── 001-ruinetwork.patch  ← Patch files
│   ├── 002-example.patch
│   ├── README.md             ← This documentation
│   └── update-patches-json.sh ← Update script
└── ...
```

**Update workflow for maintainers:**

1. Create/modify patch files
2. Run `./update-patches-json.sh`
3. Commit both patches and `patches.json`
4. Push to GitHub
5. Users can now fetch updates

---

**End of Documentation**

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
