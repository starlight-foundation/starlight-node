import os

# Walk through the src directory and print the fully qualified path of each file
for root, dirs, files in os.walk("src"):
    for file in files:
        print(os.path.join(root, file)[4:])
