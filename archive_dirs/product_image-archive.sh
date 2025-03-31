#!/bin/bash

# Script to move all files from product image directories into a flat archive directory
# Processes directories in chronological order so newer files overwrite older ones
# Usage: ./archive_products.sh [product_id] [custom_archive_dir]
#
# Example: ./archive_products.sh wish
#   - Will process all product_images-wish-* directories
#   - Will use product_images-wish-archive as the default archive directory

# Check if product_id parameter is provided
if [ -z "$1" ]; then
    echo "Error: Product ID parameter is required."
    echo "Usage: $0 [product_id] [custom_archive_dir]"
    echo "Example: $0 wish"
    exit 1
fi

PRODUCT_ID="$1"
PATTERN="product_images-${PRODUCT_ID}-202*"
DEFAULT_ARCHIVE="product_images-${PRODUCT_ID}-archive"

# Set archive directory (use custom if provided, otherwise use default)
ARCHIVE_DIR="${2:-$DEFAULT_ARCHIVE}"

# Create archive directory if it doesn't exist
if [ ! -d "$ARCHIVE_DIR" ]; then
    echo "Creating archive directory: $ARCHIVE_DIR"
    mkdir -p "$ARCHIVE_DIR"
fi

# Find all matching product image directories and sort them by date in the directory name
echo "Finding product image directories for '${PRODUCT_ID}'..."
DIRS=$(find . -maxdepth 1 -type d -name "$PATTERN" | sort)

# Check if any directories were found
if [ -z "$DIRS" ]; then
    echo "No product image directories found matching pattern '$PATTERN'!"
    exit 1
fi

# Print summary before moving
echo "Files will be moved from the following directories in this order (oldest to newest):"
echo "$DIRS" | sed 's/^\.\///'
echo ""
echo "Total space used by these directories:"
du -sh $(echo "$DIRS" | tr '\n' ' ')

# Count total files to be moved
total_files=0
for dir in $DIRS; do
    dir_files=$(find "$dir" -maxdepth 1 -type f | wc -l)
    total_files=$((total_files + dir_files))
done
echo "Total files to be moved: $total_files"

echo "Note: Files with duplicate names will be overwritten by newer versions."

# Ask for confirmation
read -p "Do you want to proceed? (y/n): " confirm
if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
    echo "Operation cancelled."
    exit 0
fi

# Move files from each directory into the flat archive directory
echo "Moving files to $ARCHIVE_DIR..."
moved_count=0
overwrite_count=0

for dir in $DIRS; do
    dir_name=$(basename "$dir")
    echo "Moving files from $dir_name..."

    # Count files in directory for reporting
    file_count=$(find "$dir" -maxdepth 1 -type f | wc -l)
    dir_moved=0
    dir_overwrite=0

    # Move only files to the flat archive directory
    for file in "$dir"/*; do
        if [ -f "$file" ]; then
            filename=$(basename "$file")

            # Check if file already exists in destination
            if [ -f "$ARCHIVE_DIR/$filename" ]; then
                dir_overwrite=$((dir_overwrite + 1))
            fi

            mv "$file" "$ARCHIVE_DIR/"
            if [ $? -eq 0 ]; then
                dir_moved=$((dir_moved + 1))
            fi
        fi
    done

    moved_count=$((moved_count + dir_moved))
    overwrite_count=$((overwrite_count + dir_overwrite))
    echo "  Moved $dir_moved files from $dir_name (overwrote $dir_overwrite files)"
done

echo ""
echo "Operation completed."
echo "Successfully moved $moved_count files to $ARCHIVE_DIR"
echo "Total files overwritten: $overwrite_count"

# Print disk space comparison
echo ""
echo "Disk space in archive directory:"
du -sh "$ARCHIVE_DIR"
echo ""
echo "Remaining disk space in original directories:"
du -sh $(echo "$DIRS" | tr '\n' ' ')
