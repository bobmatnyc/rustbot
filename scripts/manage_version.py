#!/usr/bin/env python3
"""
Rustbot Version Management Script

Automates version bumping across all required files:
- src/version.rs
- Cargo.toml
- VERSION_MANAGEMENT.md

Usage:
    ./scripts/manage_version.py bump patch    # 0.2.6 -> 0.2.7
    ./scripts/manage_version.py bump minor    # 0.2.6 -> 0.3.0
    ./scripts/manage_version.py bump major    # 0.2.6 -> 1.0.0
    ./scripts/manage_version.py bump build    # 0001 -> 0002
    ./scripts/manage_version.py show          # Display current version
"""

import re
import sys
from pathlib import Path
from typing import Tuple


class VersionManager:
    def __init__(self, project_root: Path):
        self.project_root = project_root
        self.version_rs = project_root / "src" / "version.rs"
        self.cargo_toml = project_root / "Cargo.toml"
        self.version_md = project_root / "VERSION_MANAGEMENT.md"

    def get_current_version(self) -> Tuple[str, str]:
        """Read current version and build from version.rs"""
        content = self.version_rs.read_text()

        version_match = re.search(r'pub const VERSION: &str = "([^"]+)"', content)
        build_match = re.search(r'pub const BUILD: &str = "([^"]+)"', content)

        if not version_match or not build_match:
            raise ValueError("Could not parse version from version.rs")

        return version_match.group(1), build_match.group(1)

    def parse_version(self, version: str) -> Tuple[int, int, int]:
        """Parse semantic version string into (major, minor, patch)"""
        parts = version.split('.')
        if len(parts) != 3:
            raise ValueError(f"Invalid version format: {version}")
        return tuple(map(int, parts))

    def bump_version(self, bump_type: str) -> Tuple[str, str]:
        """Bump version based on type (major, minor, patch, build)"""
        current_version, current_build = self.get_current_version()
        major, minor, patch = self.parse_version(current_version)

        if bump_type == "major":
            major += 1
            minor = 0
            patch = 0
            build = "0001"
        elif bump_type == "minor":
            minor += 1
            patch = 0
            build = "0001"
        elif bump_type == "patch":
            patch += 1
            build = "0001"
        elif bump_type == "build":
            build_num = int(current_build)
            build = f"{build_num + 1:04d}"
            return current_version, build
        else:
            raise ValueError(f"Invalid bump type: {bump_type}")

        new_version = f"{major}.{minor}.{patch}"
        return new_version, build

    def update_version_rs(self, version: str, build: str):
        """Update src/version.rs"""
        content = self.version_rs.read_text()

        content = re.sub(
            r'pub const VERSION: &str = "[^"]+"',
            f'pub const VERSION: &str = "{version}"',
            content
        )
        content = re.sub(
            r'pub const BUILD: &str = "[^"]+"',
            f'pub const BUILD: &str = "{build}"',
            content
        )

        self.version_rs.write_text(content)
        print(f"✓ Updated {self.version_rs}")

    def update_cargo_toml(self, version: str):
        """Update Cargo.toml"""
        content = self.cargo_toml.read_text()

        content = re.sub(
            r'^version = "[^"]+"',
            f'version = "{version}"',
            content,
            flags=re.MULTILINE
        )

        self.cargo_toml.write_text(content)
        print(f"✓ Updated {self.cargo_toml}")

    def update_version_md(self, version: str, build: str):
        """Update VERSION_MANAGEMENT.md"""
        content = self.version_md.read_text()

        # Update the Current Version section
        pattern = r'## Current Version\s+- \*\*Version\*\*: [^\n]+\s+- \*\*Build\*\*: [^\n]+'
        replacement = f'## Current Version\n\n- **Version**: {version}\n- **Build**: {build}'

        content = re.sub(pattern, replacement, content)

        self.version_md.write_text(content)
        print(f"✓ Updated {self.version_md}")

    def bump(self, bump_type: str):
        """Execute version bump"""
        old_version, old_build = self.get_current_version()
        new_version, new_build = self.bump_version(bump_type)

        print(f"\n📦 Version Bump: {bump_type}")
        print(f"   Old: v{old_version}-{old_build}")
        print(f"   New: v{new_version}-{new_build}\n")

        # Update all files
        self.update_version_rs(new_version, new_build)

        # Only update Cargo.toml if version changed (not just build)
        if new_version != old_version:
            self.update_cargo_toml(new_version)

        self.update_version_md(new_version, new_build)

        print(f"\n✅ Version bumped successfully!")
        print(f"\nNext steps:")
        print(f"1. Review changes: git diff")
        print(f"2. Rebuild: cargo build")
        print(f"3. Commit: git add -A && git commit -m 'chore: bump version to {new_version}'")
        print(f"4. Push: git push origin main")

    def show(self):
        """Display current version"""
        version, build = self.get_current_version()
        print(f"\n📦 Current Version")
        print(f"   Version: {version}")
        print(f"   Build: {build}")
        print(f"   Full: v{version}-{build}\n")


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    command = sys.argv[1]

    # Find project root (directory containing Cargo.toml)
    project_root = Path(__file__).parent.parent
    if not (project_root / "Cargo.toml").exists():
        print("Error: Could not find Cargo.toml")
        sys.exit(1)

    manager = VersionManager(project_root)

    try:
        if command == "show":
            manager.show()
        elif command == "bump":
            if len(sys.argv) < 3:
                print("Error: bump command requires type (major|minor|patch|build)")
                print(__doc__)
                sys.exit(1)
            bump_type = sys.argv[2]
            manager.bump(bump_type)
        else:
            print(f"Error: Unknown command '{command}'")
            print(__doc__)
            sys.exit(1)
    except Exception as e:
        print(f"\n❌ Error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
