# Mac Setup Guide for Oxidized Oasis Web Sands

This document provides step-by-step instructions for setting up all dependencies required to run the Oxidized Oasis Web Sands project on macOS.

## Prerequisites

Before starting, ensure you have:
- macOS 10.15 (Catalina) or later
- Terminal access
- Internet connection

## Step 1: Install Xcode Command Line Tools

Required for compiling Rust and native dependencies.

```bash
xcode-select --install
```

Click "Install" when the dialog appears, and wait for the installation to complete.

## Step 2: Install Rust

The project requires Rust for both backend and frontend development.

```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Select option 1 (default installation) when prompted

# Reload your shell configuration
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

## Step 3: Add WebAssembly Target

The frontend uses WebAssembly, so we need to add the WASM target.

```bash
# Add the WebAssembly target
rustup target add wasm32-unknown-unknown

# Verify the target was added
rustup target list --installed
```

## Step 4: Install Trunk

Trunk is used to build and serve the frontend WebAssembly application.

```bash
# Install Trunk
cargo install trunk

# Verify installation
trunk --version
```

## Step 5: Install PostgreSQL

The project uses PostgreSQL as its database.

```bash
# Install PostgreSQL using Homebrew (install Homebrew first if needed)
# If you don't have Homebrew: /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

brew install postgresql

# Start PostgreSQL service
brew services start postgresql

# Verify installation
psql --version

# Create a default database
createdb $(whoami)
```

## Step 6: Install SQLx CLI

SQLx CLI is used for database migrations and management.

```bash
# Install sqlx-cli with PostgreSQL support
cargo install sqlx-cli --features postgres

# Verify installation
sqlx --version
```

## Step 7: Environment Setup

Create necessary environment variables and configuration files.

```bash
# Navigate to your project directory
cd /Users/dreamer/Codespaces/Oasis

# Create a .env file for environment variables (if it doesn't exist)
touch .env

# Add the following to your .env file:
echo "DATABASE_URL=postgresql://$(whoami)@localhost/oxidized_oasis" >> .env
echo "JWT_SECRET=your-super-secret-jwt-key-change-this-in-production" >> .env
echo "SMTP_HOST=smtp.gmail.com" >> .env
echo "SMTP_PORT=587" >> .env
echo "SMTP_USERNAME=your-email@gmail.com" >> .env
echo "SMTP_PASSWORD=your-app-password" >> .env
```

## Step 8: Database Setup

Set up the PostgreSQL database and run migrations.

```bash
# Create the database
sqlx database create

# Run database migrations
sqlx migrate run

# Verify database setup
sqlx migrate info
```

## Step 9: Install Project Dependencies

Install all Rust dependencies for both backend and frontend.

```bash
# Install backend dependencies (from project root)
cargo build

# Install frontend dependencies
cd frontend
cargo build --target wasm32-unknown-unknown
cd ..
```

## Step 10: Verify Installation

Test that everything is working correctly.

```bash
# Build the entire project
cargo build --workspace

# Build the frontend
cd frontend
trunk build
cd ..
```

## Running the Application

Once everything is installed:

### Start the Backend Server
```bash
# From the project root directory
cargo run
```

The backend will start on `http://localhost:8080` (or the port specified in your configuration).

### Start the Frontend Development Server
```bash
# In a new terminal, navigate to the frontend directory
cd frontend

# Start the frontend development server
trunk serve
```

The frontend will start on `http://localhost:8000` and will automatically reload when you make changes.

## Troubleshooting

### Common Issues and Solutions

1. **"command not found: cargo"**
   - Restart your terminal or run `source ~/.cargo/env`

2. **PostgreSQL connection errors**
   - Make sure PostgreSQL is running: `brew services restart postgresql`
   - Check your DATABASE_URL in the .env file

3. **Trunk build fails**
   - Ensure you have the wasm32-unknown-unknown target: `rustup target add wasm32-unknown-unknown`

4. **Permission denied errors**
   - You may need to use `sudo` for some installations
   - Make sure your user has proper permissions

5. **SSL/TLS errors during compilation**
   - The project includes vendored OpenSSL for macOS, but you might need: `brew install openssl`

### Updating Dependencies

To update all dependencies:

```bash
# Update Rust
rustup update

# Update Trunk
cargo install trunk --force

# Update SQLx CLI
cargo install sqlx-cli --features postgres --force

# Update Cargo dependencies
cargo update
```

## System Requirements

- **Minimum macOS Version:** 10.15 (Catalina)
- **Recommended macOS Version:** 12.0 (Monterey) or later
- **RAM:** 8GB minimum, 16GB recommended
- **Storage:** 2GB free space for all dependencies
- **Architecture:** Works on both Intel and Apple Silicon Macs

## Additional Tools (Optional)

These tools can enhance your development experience:

```bash
# Install a PostgreSQL GUI client
brew install --cask pgadmin4

# Install a better terminal (optional)
brew install --cask iterm2

# Install a code editor with Rust support (if not using VS Code)
brew install --cask visual-studio-code
```

## Next Steps

After completing this setup:

1. Read the project documentation in the `docs/` directory
2. Explore the codebase structure
3. Review the API endpoints and frontend components
4. Start contributing to the project!

## Support

If you encounter issues not covered in this guide:

1. Check the project's issue tracker
2. Review the official Rust and PostgreSQL documentation
3. Ask for help in the project's communication channels

---

**Last Updated:** July 23, 2025
**Compatible with:** macOS 10.15+ (Intel and Apple Silicon)