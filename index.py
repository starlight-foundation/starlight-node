# Print out the full pathnames of all files in the src/ directory
import os

for root, dirs, files in os.walk("src/"):
    for file in files:
        print(os.path.join(root, file)[4:])
