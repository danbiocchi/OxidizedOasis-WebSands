#!/bin/bash

# OxidizedOasis-WebSands Mac Setup Script
# This script automates the complete setup process for Mac development environment

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${CYAN}========================================${NC}"
    echo -e "${CYAN}$1${NC}"
    echo -e "${CYAN}========================================${NC}"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check if a Homebrew package is installed
brew_package_installed() {
    brew list "$1" >/dev/null 2>&1
}

# Function to install Homebrew if not present
install_homebrew() {
    if ! command_exists brew; then
        print_status "Installing Homebrew..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        
        # Add Homebrew to PATH for current session
        if [[ $(uname -m) == "arm64" ]]; then
            eval "$(/opt/homebrew/bin/brew shellenv)"
        else
            eval "$(/usr/local/bin/brew shellenv)"
        fi
        
        print_success "Homebrew installed successfully!"
    else
        print_success "Homebrew already installed"
    fi
}

# Function to install Xcode Command Line Tools
install_xcode_tools() {
    if ! xcode-select -p >/dev/null 2>&1; then
        print_status "Installing Xcode Command Line Tools..."
        xcode-select --install
        
        print_warning "Please complete the Xcode Command Line Tools installation in the popup window."
        print_warning "Press Enter after the installation is complete to continue..."
        read -r
        
        # Verify installation
        if xcode-select -p >/dev/null 2>&1; then
            print_success "Xcode Command Line Tools installed successfully!"
        else
            print_error "Xcode Command Line Tools installation failed!"
            exit 1
        fi
    else
        print_success "Xcode Command Line Tools already installed"
    fi
}

# Function to install Rust
install_rust() {
    if ! command_exists rustc; then
        print_status "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source ~/.cargo/env
        print_success "Rust installed successfully!"
    else
        print_success "Rust already installed ($(rustc --version))"
    fi
    
    # Add WebAssembly target
    print_status "Adding WebAssembly target..."
    rustup target add wasm32-unknown-unknown
    print_success "WebAssembly target added!"
}

# Function to install Trunk
install_trunk() {
    if ! command_exists trunk; then
        print_status "Installing Trunk..."
        cargo install trunk
        print_success "Trunk installed successfully!"
    else
        print_success "Trunk already installed ($(trunk --version))"
    fi
}

# Function to install PostgreSQL
install_postgresql() {
    if ! brew_package_installed postgresql@14; then
        print_status "Installing PostgreSQL 14..."
        brew install postgresql@14
        
        # Start PostgreSQL service
        print_status "Starting PostgreSQL service..."
        brew services start postgresql@14
        
        # Add PostgreSQL to PATH
        echo 'export PATH="/usr/local/opt/postgresql@14/bin:$PATH"' >> ~/.zshrc
        echo 'export PATH="/opt/homebrew/opt/postgresql@14/bin:$PATH"' >> ~/.zshrc
        export PATH="/usr/local/opt/postgresql@14/bin:$PATH"
        export PATH="/opt/homebrew/opt/postgresql@14/bin:$PATH"
        
        print_success "PostgreSQL 14 installed and started!"
    else
        print_success "PostgreSQL 14 already installed"
        
        # Ensure service is running
        if ! brew services list | grep postgresql@14 | grep started >/dev/null; then
            print_status "Starting PostgreSQL service..."
            brew services start postgresql@14
        fi
    fi
}

# Function to install SQLx CLI
install_sqlx_cli() {
    if ! command_exists sqlx; then
        print_status "Installing SQLx CLI..."
        cargo install sqlx-cli --no-default-features --features native-tls,postgres
        print_success "SQLx CLI installed successfully!"
    else
        print_success "SQLx CLI already installed ($(sqlx --version))"
    fi
}

# Function to setup database
setup_database() {
    print_status "Setting up database..."
    
    # Check if .env file exists
    if [[ ! -f .env ]]; then
        print_error ".env file not found! Please create it with the necessary environment variables."
        print_status "You can use the example from the README or Mac_Setup.md as a template."
        exit 1
    fi
    
    # Source environment variables
    export $(grep -v '^#' .env | xargs)
    
    # Check if database exists, if not create it
    if ! psql -U "$(whoami)" -d postgres -lqt | cut -d \| -f 1 | grep -qw "$DB_NAME"; then
        print_status "Creating database: $DB_NAME"
        psql -U "$(whoami)" -d postgres -c "CREATE DATABASE $DB_NAME;"
        print_success "Database $DB_NAME created!"
    else
        print_success "Database $DB_NAME already exists"
    fi
    
    # Run migrations
    print_status "Running database migrations..."
    sqlx migrate run
    
    if [[ $? -eq 0 ]]; then
        print_success "Database migrations completed successfully!"
    else
        print_error "Database migrations failed!"
        exit 1
    fi
}

# Function to install project dependencies
install_dependencies() {
    print_status "Installing project dependencies..."
    cargo build
    print_success "Project dependencies installed successfully!"
}

# Function to test the setup
test_setup() {
    print_status "Testing the setup..."
    
    # Test backend build
    print_status "Testing backend build..."
    if cargo check; then
        print_success "Backend builds successfully!"
    else
        print_error "Backend build failed!"
        return 1
    fi
    
    # Test frontend build
    print_status "Testing frontend build..."
    cd frontend
    if trunk build; then
        print_success "Frontend builds successfully!"
    else
        print_error "Frontend build failed!"
        cd ..
        return 1
    fi
    cd ..
    
    print_success "All tests passed!"
}

# Function to create a simple build script for Mac
create_build_script() {
    print_status "Creating Mac build script..."
    
    cat > build-mac.sh << 'EOF'
#!/bin/bash

# OxidizedOasis-WebSands Mac Build Script
# Build frontend and run backend

set -e

echo "Building frontend..."
cd frontend
trunk build
cd ..

echo "Starting backend server..."
cargo run
EOF
    
    chmod +x build-mac.sh
    print_success "Mac build script created: build-mac.sh"
}

# Main setup function
main() {
    print_header "OxidizedOasis-WebSands Mac Setup"
    print_status "Starting automated Mac development environment setup..."
    
    # Check if running on macOS
    if [[ "$OSTYPE" != "darwin"* ]]; then
        print_error "This script is designed for macOS only!"
        exit 1
    fi
    
    # Check if we're in the right directory
    if [[ ! -f "Cargo.toml" ]] || [[ ! -d "frontend" ]]; then
        print_error "Please run this script from the project root directory!"
        exit 1
    fi
    
    print_header "Step 1: Installing Homebrew"
    install_homebrew
    
    print_header "Step 2: Installing Xcode Command Line Tools"
    install_xcode_tools
    
    print_header "Step 3: Installing Rust & WebAssembly Support"
    install_rust
    
    print_header "Step 4: Installing Trunk"
    install_trunk
    
    print_header "Step 5: Installing PostgreSQL"
    install_postgresql
    
    print_header "Step 6: Installing SQLx CLI"
    install_sqlx_cli
    
    print_header "Step 7: Setting up Database"
    setup_database
    
    print_header "Step 8: Installing Project Dependencies"
    install_dependencies
    
    print_header "Step 9: Creating Build Script"
    create_build_script
    
    print_header "Step 10: Testing Setup"
    if test_setup; then
        print_header "🎉 SETUP COMPLETE! 🎉"
        print_success "Your Mac development environment is ready!"
        echo ""
        print_status "To start the application:"
        echo "  ./build-mac.sh"
        echo ""
        print_status "Or manually:"
        echo "  Backend: cargo run"
        echo "  Frontend: cd frontend && trunk serve"
        echo ""
        print_status "Application will be available at: http://localhost:8080"
        echo ""
        print_warning "Note: You may need to restart your terminal or run 'source ~/.zshrc' for PATH changes to take effect."
    else
        print_error "Setup completed with errors. Please check the output above."
        exit 1
    fi
}

# Run main function
main "$@"