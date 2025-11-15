# Session Log: Fix Panic During App Initialization

**Date**: 2025-11-13
**Issue**: Application crashed on startup with `EXC_CRASH (SIGABRT)`
**Status**: ✅ Resolved

## Problem Summary

Rustbot was crashing immediately on startup with a panic abort. The crash report showed:

```
Exception Type: EXC_CRASH (SIGABRT)
Termination Reason: Namespace SIGNAL, Code 6 Abort trap: 6
Application Specific Information: abort() called
```

### Root Cause Analysis

The crash occurred in the FFI boundary during macOS's `applicationDidFinishLaunching` callback. Stack trace analysis revealed:

1. **Frame 52** (`main.rs:40`): Panic originated in main initialization
2. **Frame 12**: `winit::platform_impl::macos::app_state::ApplicationDelegate::app_did_finish_launching`
3. **Frame 11**: `core::panicking::panic_cannot_unwind` - **Panic in FFI context where unwinding isn't allowed**

The specific issue was on line 84 of `src/main.rs`:

```rust
let api_key = std::env::var("OPENROUTER_API_KEY")
    .expect("OPENROUTER_API_KEY not found in .env.local");
```

**Why it crashed**:
- The `.expect()` call would panic if `OPENROUTER_API_KEY` wasn't found in environment
- This panic occurred inside `eframe::run_native`'s initialization callback
- The callback is invoked from macOS FFI (`applicationDidFinishLaunching`)
- Rust cannot unwind through FFI boundaries, so it calls `abort()` instead
- Result: immediate application termination with `SIGABRT`

**Contributing factors**:
- `.env.local` loading used `.ok()` which silently ignored errors
- No fallback if `.env.local` wasn't found in current directory
- When app is double-clicked (vs run from terminal), working directory may differ

## Solution Implemented

### 1. Replaced `.expect()` with Graceful Error Handling

**Before** (`src/main.rs:84`):
```rust
let api_key = std::env::var("OPENROUTER_API_KEY")
    .expect("OPENROUTER_API_KEY not found in .env.local");
```

**After** (`src/main.rs:84-100`):
```rust
let api_key = match std::env::var("OPENROUTER_API_KEY") {
    Ok(key) => key,
    Err(_) => {
        // Log error for debugging
        tracing::error!("OPENROUTER_API_KEY not found in environment");
        tracing::error!("Please ensure .env.local file exists with OPENROUTER_API_KEY=your_key");

        // Display error message to user and exit gracefully
        eprintln!("\n❌ ERROR: Missing OPENROUTER_API_KEY environment variable\n");
        eprintln!("Please create a .env.local file in the project directory with:");
        eprintln!("OPENROUTER_API_KEY=your_api_key_here\n");
        eprintln!("Get your API key from: https://openrouter.ai/keys\n");

        // Exit gracefully instead of panicking in FFI boundary
        std::process::exit(1);
    }
};
```

### 2. Improved `.env.local` Loading Robustness

**Before** (`src/main.rs:30`):
```rust
dotenvy::from_filename(".env.local").ok();
```

**After** (`src/main.rs:29-44`):
```rust
// Load .env.local file - try multiple locations for robustness
// First try current directory, then executable directory
let env_loaded = dotenvy::from_filename(".env.local").is_ok()
    || if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            dotenvy::from_path(exe_dir.join(".env.local")).is_ok()
        } else {
            false
        }
    } else {
        false
    };

if !env_loaded {
    tracing::warn!(".env.local file not found - will need OPENROUTER_API_KEY from environment");
}
```

**Benefits**:
- Tries multiple locations for `.env.local` file
- Logs warning if file not found (helps debugging)
- Gracefully continues if environment variable is set through other means

## Files Modified

1. **src/main.rs** (lines 25-44, 84-100):
   - Added robust `.env.local` loading with fallback to executable directory
   - Replaced `.expect()` panic with graceful error handling and `std::process::exit(1)`
   - Added helpful error messages for missing API key

## Testing Performed

### Before Fix
```bash
$ ./target/debug/rustbot
# Result: Immediate crash with SIGABRT
```

### After Fix
```bash
$ ./target/debug/rustbot
[2025-11-14T03:36:44.859551Z INFO  rustbot::agent::loader] Loaded agent 'assistant' from "agents/presets/assistant.json"
[2025-11-14T03:36:44.859620Z INFO  rustbot::agent::loader] Loaded agent 'web_search' from "agents/presets/web_search.json"
# Result: ✅ SUCCESS - Application runs successfully
```

### Test without .env.local
If `.env.local` is missing or `OPENROUTER_API_KEY` is not set, the app now:
1. Logs detailed error to tracing
2. Displays user-friendly error message
3. Exits gracefully with code 1 (instead of crashing with SIGABRT)

## Technical Details

### Why FFI Panics Cause Aborts

Rust's panic unwinding mechanism uses stack unwinding to clean up resources. However:

1. **FFI boundaries** (Foreign Function Interface) between Rust and C/Objective-C don't support unwinding
2. When a panic tries to unwind through FFI, it's **undefined behavior**
3. Rust's safety guarantee: call `abort()` instead of risking undefined behavior
4. Result: immediate process termination with `SIGABRT`

### macOS Application Lifecycle

```
macOS Launch
    ↓
dyld loads binary
    ↓
main() called
    ↓
eframe::run_native()
    ↓
NSApplication initialization
    ↓
applicationDidFinishLaunching ← FFI BOUNDARY
    ↓
Initialization callback (Rust closure) ← PANIC HERE = ABORT
    ↓
Window creation
```

If panic occurs in the initialization callback, it's inside FFI context, so:
- ✅ `std::process::exit(1)` - Safe, graceful exit
- ❌ `.expect()` / `panic!()` - Triggers abort via FFI

## Key Learnings

1. **Never use `.expect()` or `panic!()` in eframe/egui initialization callbacks**
   - These callbacks are invoked from FFI (macOS/Windows/Linux windowing system)
   - Panics cannot unwind through FFI and will abort the process

2. **Always handle errors gracefully in FFI contexts**
   - Use `match` or `if let` instead of `.expect()`
   - Return `Result` or call `std::process::exit()` for fatal errors
   - Provide helpful error messages to users

3. **Environment variable loading needs robustness**
   - Don't assume current working directory
   - Try multiple locations (current dir, executable dir, system paths)
   - Log warnings when files aren't found
   - Validate environment variables are set before using them

4. **Debugging FFI panics**
   - Look for `panic_cannot_unwind` in stack traces
   - Check for application lifecycle callbacks (e.g., `applicationDidFinishLaunching`)
   - Identify Rust code called from C/Objective-C/Swift

## Future Improvements

Potential enhancements for even better robustness:

1. **GUI error dialog** instead of console error
   - Use native OS dialog to show error if API key is missing
   - Better UX when app is double-clicked from Finder

2. **Settings window** for API key entry
   - Allow users to enter API key through GUI
   - Save to system keychain for security

3. **Environment variable fallback chain**
   - Try `OPENROUTER_API_KEY` first
   - Fall back to `RUSTBOT_API_KEY`
   - Check macOS keychain as final fallback

4. **Comprehensive startup validation**
   - Check all required resources before creating window
   - Pre-validate fonts, icons, configs
   - Show single consolidated error if multiple issues found

## Conclusion

The crash was caused by a panic in an FFI boundary where unwinding is not allowed. The fix replaces panicking code with graceful error handling, ensuring the application either starts successfully or exits cleanly with a helpful error message.

**Result**: Application now starts reliably without crashing. Users receive clear feedback if configuration is missing.
