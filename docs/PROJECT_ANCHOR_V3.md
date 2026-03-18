# Project AnchoR v3: Transactional Surgical Remediation

## 1. Objective

To provide a zero-error, byte-perfect self-healing mechanism for documentation that has drifted from its observed source code.

## 2. Core Mechanisms

### 2.1 Byte-Range Encapsulation (`ByteRange`)

- **Protocol**: All surgical operations MUST use the `ByteRange` struct instead of `(usize, usize)` tuples.
- **Safety**: Includes `overlaps()`, `contains()`, and `is_valid_for(file_size)` to prevent out-of-bounds corruption during multi-fix applications.

### 2.2 Immutable Cursor Positioning (v3.3)

- **Algorithm**: Uses `split_inclusive('\n')` to preserve exact physical line endings (LF/CRLF).
- **Alignment**: Automatically detects and mirrors the source file's line-ending style in the replacement payload.

### 2.3 CAS (Compare-And-Swap) Verification

- **One-Time Hashing**: Uses **Blake3** for whole-file integrity checks before applying a batch of fixes.
- **Content-at-Range Mirroring**: Each `SurgicalFix` verifies that the current bytes at the target range match the `original_content` exactly before swapping.

## 3. Usage & CLI

- Command: `wendao fix <target> --confidence <threshold>`
- Exit Codes:
  - `0`: Success or no issues found.
  - `101`: CAS Mismatch (protection triggered).
