#!/bin/bash

# A script to start the Qdrant Docker container for Roo Code embeddings.
#
# It checks if the container is running, starts it if it's stopped,
# or creates it if it doesn't exist.

# --- Configuration ---
# The name we give to our container for easy reference.
CONTAINER_NAME="qdrant-search"
IMAGE_NAME="qdrant/qdrant"

# --- Script Logic ---

# Use 'docker ps' to see running containers.
# The 'grep' command filters the output to find our specific container.
# The '-q' flag tells grep to be quiet and just return an exit code.
if docker ps | grep -q " ${CONTAINER_NAME}$"; then
  # If grep finds a match, the container is already running.
  echo "✅ Qdrant is already up and running."
  exit 0
fi

# If we're here, the container isn't running.
# Now, check if a container with that name exists but is stopped.
if docker ps -a | grep -q " ${CONTAINER_NAME}$"; then
  # If it exists, we just need to start it.
  echo "🟡 Qdrant container is stopped. Starting it now..."
  docker start "$CONTAINER_NAME"
  echo "✅ Qdrant has been started."
  exit 0
fi

# If we're here, the container doesn't exist at all.
# So, we create it for the first time with the recommended settings.
echo "🚀 Qdrant container not found. Creating and starting a new one..."
docker run -d \
  -p 6333:6333 \
  --name "$CONTAINER_NAME" \
  --restart unless-stopped \
  "$IMAGE_NAME"

echo "✅ Qdrant has been created and is now running."