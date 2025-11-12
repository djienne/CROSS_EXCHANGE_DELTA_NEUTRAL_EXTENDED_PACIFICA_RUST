#!/usr/bin/env python3
"""
Create a portable ZIP archive of the Extended DEX connector project.
Excludes large build artifacts and non-portable files.
"""

import os
import zipfile
from pathlib import Path

# Directories and files to exclude
EXCLUDE_DIRS = {
    'target',           # Rust build artifacts (very large)
    '.git',             # Git repository data
    '.vscode',          # VS Code settings
    '.idea',            # IntelliJ IDEA settings
    '__pycache__',      # Python cache
    'node_modules',     # Node modules if any
    '.pytest_cache',    # Pytest cache
}

EXCLUDE_FILES = {
    '.DS_Store',        # macOS
    'Thumbs.db',        # Windows
    '*.zip',            # Existing archives
}

EXCLUDE_EXTENSIONS = {
    '.pyc',
    '.pyo',
    '.pyd',
    '.so',
    '.dylib',
    '.dll',
}

def should_exclude(path: Path, root_path: Path) -> bool:
    """Check if a path should be excluded from the archive."""
    # Check if any parent directory is in exclude list
    try:
        relative = path.relative_to(root_path)
        for part in relative.parts:
            if part in EXCLUDE_DIRS:
                return True
    except ValueError:
        pass

    # Check filename
    if path.name in EXCLUDE_FILES:
        return True

    # Check extension
    if path.suffix in EXCLUDE_EXTENSIONS:
        return True

    # Exclude zip files
    if path.suffix == '.zip':
        return True

    return False

def create_project_archive(project_dir: str = '.', output_name: str = None) -> str:
    """
    Create a ZIP archive of the project.

    Args:
        project_dir: Path to the project directory
        output_name: Name for the output ZIP file (without extension)

    Returns:
        Path to the created archive
    """
    project_path = Path(project_dir).resolve()

    # Generate output filename
    if output_name is None:
        output_name = 'extended_pacifica_delta_neutral'

    zip_path = project_path / f'{output_name}.zip'

    # Count files for progress
    total_files = 0
    added_files = 0

    print(f"Creating archive: {zip_path.name}")
    print(f"Source directory: {project_path}")
    print()

    with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED) as zipf:
        # Walk through all files in the project
        for root, dirs, files in os.walk(project_path):
            root_path = Path(root)

            # Filter out excluded directories (modifies dirs in-place)
            dirs[:] = [d for d in dirs if d not in EXCLUDE_DIRS]

            for file in files:
                file_path = root_path / file
                total_files += 1

                # Skip excluded files
                if should_exclude(file_path, project_path):
                    print(f"  [SKIP] {file_path.relative_to(project_path)}")
                    continue

                # Add file to archive
                arcname = file_path.relative_to(project_path)
                try:
                    zipf.write(file_path, arcname)
                    added_files += 1
                    print(f"  [ADD]  {arcname}")
                except (ValueError, OSError) as e:
                    print(f"  [ERROR] Failed to add {arcname}: {e}")

    # Get archive size
    archive_size = zip_path.stat().st_size
    size_mb = archive_size / (1024 * 1024)

    print()
    print("=" * 60)
    print(f"Archive created successfully!")
    print(f"Location: {zip_path}")
    print(f"Size: {size_mb:.2f} MB")
    print(f"Files added: {added_files} / {total_files} total")
    print("=" * 60)

    return str(zip_path)

if __name__ == '__main__':
    import argparse

    parser = argparse.ArgumentParser(
        description='Create a portable ZIP archive of the Extended connector project'
    )
    parser.add_argument(
        '--dir',
        default='.',
        help='Project directory to archive (default: current directory)'
    )
    parser.add_argument(
        '--output',
        help='Output filename without extension (default: extended_connector)'
    )

    args = parser.parse_args()

    try:
        archive_path = create_project_archive(args.dir, args.output)
        print(f"\n[SUCCESS] Archive ready: {archive_path}")
    except Exception as e:
        print(f"\n[ERROR] Error creating archive: {e}")
        exit(1)
