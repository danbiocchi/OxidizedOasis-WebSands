# OxidizedOasis-WebSands

OxidizedOasis-WebSands is a robust, high-performance web application built with Rust, focusing on efficient user management and authentication. 

<p align="center">
    <img src="docs/images/homepic.png" alt="Home Page Screenshot" width="600">
    <br></br>
    <img src="docs/images/loginpic.png" alt="Login Page Screenshot" width="600">
    <br></br>
    <img src="docs/images/registerpic.png" alt="Register Page Screenshot" width="600">
    <br></br>
    <img src="docs/images/verify.png" alt="Verify Email" width="600">
    <br></br>
    <img src="docs/images/login_dashboard.png" alt="Dashboard Screenshot" width="600">
    <br></br>
    <img src="docs/images/chat.png" alt="Chat Interface Screenshot" width="600">
    <br></br>
    <img src="docs/images/data.png" alt="Data Visualization Screenshot" width="600">
    <br></br>
    <img src="docs/images/system_logs.png" alt="System Log Screenshot" width="600">
    <br></br>
    <img src="docs/images/edit_user.png" alt="Edit User Screenshot" width="600">
    <br></br>
    <img src="docs/images/log_details.png" alt="Log Details Screenshot" width="600">
    <br></br>
    <img src="docs/images/security_logs.png" alt="Security Logs Screenshot" width="600">
    <br></br>
</p>

## 🌟 About This Project

OxidizedOasis-WebSands is designed to provide a solid foundation for building scalable web applications with strong security features. Our goal is to showcase the capabilities of Rust in creating high-performance, secure web services while maintaining excellent developer ergonomics.
This project demonstrates the power of Rust in full-stack web development, utilizing the Actix-web framework for the backend and Yew for a dynamic frontend.

## 🏗️ Architecture Overview

OxidizedOasis-WebSands follows a clean, modular architecture that separates concerns and promotes maintainability:

- **Frontend**: Built with Yew framework, providing a reactive WebAssembly-powered UI
- **Backend API**: Implemented with Actix-web, offering RESTful endpoints for all application features
- **Core Domain**: Contains business logic and domain models independent of infrastructure concerns
- **Infrastructure**: Handles database interactions, external services, and cross-cutting concerns
- **Common**: Shared utilities, constants, and types used throughout the application

This architecture enables independent development and testing of components while maintaining clear boundaries between different parts of the system.

## 📚 Documentation

Comprehensive documentation for OxidizedOasis-WebSands is available to help developers, administrators, and users understand and work with the system effectively:

- [Software Development Document](docs/Software%20Development%20Document/TABLE%20OF%20CONTENTS.md): Detailed technical specifications, architecture overview, and development guidelines.
- [Security Audit Report](docs/Security_Audit_Report.md): In-depth analysis of the project's security measures and recommendations for improvement.
- [Security Backlog](docs/Security_Backlog.md): Ongoing security tasks and improvements planned for the project.
- [Project Structure](docs/Project_Structure.md): Overview of the project's directory structure and file organization.
- [Architecture Review](docs/Architecture_Review.md): Analysis of the system architecture, design patterns, and technical decisions.

*Note: The following documentation is planned but not yet available:*
- User Guide: Guide for end-users on how to use the application.
- Logging Plan: Detailed plan for implementing comprehensive logging in the project.
- Testing Backlog: List of tests to be implemented as part of the test-driven development approach.

## ✨ Key Features

- 🔒 Advanced user authentication system with JWT (JSON Web Tokens)
- ✉️ Secure user registration with email verification
- 🔑 Secure password reset with time-limited tokens and email verification
- 🔐 Password hashing using bcrypt for enhanced security
- 🚀 High-performance database operations with SQLx and PostgreSQL
- 🛡️ Cross-Site Scripting (XSS) protection with input sanitization
- 🌐 Cross-Origin Resource Sharing (CORS) configuration for API security
- 🔍 Comprehensive input validation and error handling
- 📊 Efficient CRUD operations for user management
- 🎨 Modern, responsive frontend with Yew framework
- 📱 Mobile-friendly design for seamless user experience across devices
- 🔧 Easily extensible architecture for adding new features
- 🔄 Database migrations for easy schema management and updates
- 🧪 Test-driven development approach for improved code quality and reliability
- 🎯 BEM methodology CSS architecture for maintainable styling
- ⚡ Performance-optimized CSS with critical path rendering
- 🔄 Lazy loading for non-critical styles
- 📦 Minimized render-blocking CSS
- 💾 Browser caching optimization for styles

## 🔐 JWT Authentication System

OxidizedOasis-WebSands implements a comprehensive JWT-based authentication system with the following features:
- 🔑 Secure token generation with configurable expiration times
- 🚫 Token revocation system to immediately invalidate compromised tokens
- 📊 Active token tracking for monitoring user sessions across devices
- 🔄 Automatic token refresh mechanism to maintain seamless user experience
- 🔍 Token validation with signature verification and expiration checks
- 🔒 Role-based access control integrated with JWT claims

## 🛠️ Technology Stack

- **Backend**:
    - [Rust](https://www.rust-lang.org/) - A language empowering everyone to build reliable and efficient software
    - [Actix-web](https://actix.rs/) - A powerful, pragmatic, and extremely fast web framework for Rust
    - [SQLx](https://github.com/launchbadge/sqlx) - The Rust SQL Toolkit
    - [jsonwebtoken](https://github.com/Keats/jsonwebtoken) - JWT implementation in Rust
    - [bcrypt](https://docs.rs/bcrypt/latest/bcrypt/) - Easily hash and verify passwords using bcrypt

- **Frontend**:
    - [Yew](https://yew.rs/) - A modern web framework for creating multi-threaded frontend apps with WebAssembly
    - [Trunk](https://trunkrs.dev/) - Build, bundle & ship your Rust WASM application to the web
    - **CSS Architecture**:
        - BEM Methodology for scalable and maintainable CSS
        - Critical CSS optimization for above-the-fold content
        - Lazy loading strategies for non-critical styles
        - Performance-focused style organization
        - Browser cache optimization

- **Database**:
    - [PostgreSQL](https://www.postgresql.org/) - The World's Most Advanced Open Source Relational Database

- **Development & Deployment**:
    - [Docker](https://www.docker.com/) - For containerization and easy deployment
    - [GitHub Actions](https://github.com/features/actions) - For CI/CD pipelines

## 🚀 Getting Started

### Prerequisites

**🍎 For macOS users:** You can skip manual installation of prerequisites! Use our automated setup script:
```sh
./setup-mac.sh
```

**For manual installation or other platforms**, ensure you have the following installed:
- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [PostgreSQL](https://www.postgresql.org/download/) (version 13 or later)
- [Docker](https://docs.docker.com/get-docker/) (optional, for containerized deployment)
- [Trunk](https://trunkrs.dev/) (`cargo install trunk`)
- [wasm32-unknown-unknown target](https://rustwasm.github.io/docs/book/game-of-life/setup.html) (`rustup target add wasm32-unknown-unknown`)

### Installation

1. Clone the repository:
   ```sh
   git clone https://github.com/danbiocchi/oxidizedoasis-websands.git
   cd oxidizedoasis-websands
   ```

2. Set up the environment variables:
   Create two files in the project root: `.env` for development and `.env.test` for testing.

   `.env` file content:
   ```
   # Environment Configuration
    ENVIRONMENT=development
    PRODUCTION_URL=https://yourdomain.com
    DEVELOPMENT_URL=http://localhost:8080
    RUN_MIGRATIONS=true
    
    # Application Information
    APP_NAME=OxidizedOasis
    APP_VERSION=0.1.0
    APP_DOMAIN=yourdomain.com
    
    # Server Configuration
    SERVER_HOST=127.0.0.1
    SERVER_PORT=8080
    
    # Database Configuration
    DB_NAME=oxidizedoasis_db
    DB_HOST=localhost
    DB_USER=your_db_user
    DB_PASSWORD=your_db_password
    DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}/${DB_NAME}
    
    # Superuser Database Configuration
    SU_DB_USER=your_su_db_user
    SU_DB_PASSWORD=your_su_db_password
    SU_DATABASE_URL=postgres://${SU_DB_USER}:${SU_DB_PASSWORD}@${DB_HOST}/${DB_NAME}
    
    # JWT Configuration
    JWT_SECRET=your_jwt_secret_key
    
    # JWT Token Settings
    JWT_EXPIRATION_HOURS=24
    JWT_REFRESH_EXPIRATION_DAYS=7
    
    # Email Configuration
    SMTP_USERNAME=your_smtp_username
    SMTP_PASSWORD=your_smtp_password
    SMTP_SERVER=your_smtp_server
    FROM_EMAIL=noreply@yourdomain.com
    SUPPORT_EMAIL=support@yourdomain.com
    EMAIL_FROM_NAME="${APP_NAME} Support"
    EMAIL_VERIFICATION_SUBJECT="Verify Your ${APP_NAME} Account"
    EMAIL_PASSWORD_RESET_SUBJECT="Reset Your ${APP_NAME} Password"
    
    # Logging Configuration
    RUST_LOG=debug
    
    # Additional configurations...
   ```

   `.env.test` file content:
   ```
    # Test Environment Configuration
    ENVIRONMENT=development
    PRODUCTION_URL=https://yourdomain.com
    DEVELOPMENT_URL=http://localhost:8080
    RUN_MIGRATIONS=true
    
    # Test Application Information
    TEST_APP_NAME=OxidizedOasis-Test
    TEST_APP_VERSION=0.1.0
    TEST_APP_DOMAIN=yourdomain.com
    
    # Test Server Configuration
    TEST_SERVER_HOST=127.0.0.1
    TEST_SERVER_PORT=8080
    
    # Test Database Configuration
    TEST_DB_NAME=test_oxidizedoasis_db
    TEST_DB_HOST=localhost
    TEST_DB_USER=your_test_db_user
    TEST_DB_PASSWORD=your_test_db_password
    TEST_DATABASE_URL=postgres://${TEST_DB_USER}:${TEST_DB_PASSWORD}@${TEST_DB_HOST}/${TEST_DB_NAME}
    
    # Test Superuser Database Configuration
    TEST_DB_SUPERUSER=your_test_su_db_user
    TEST_DB_SUPERUSER_PASSWORD=your_test_su_db_password
    TEST_SU_DATABASE_URL=postgres://${TEST_DB_SUPERUSER}:${TEST_DB_SUPERUSER_PASSWORD}@${TEST_DB_HOST}/${TEST_DB_NAME}
    
    # Test JWT Configuration
    TEST_JWT_SECRET=your_test_jwt_secret_key
    
    # Test JWT Token Settings
    TEST_JWT_EXPIRATION_HOURS=24
    TEST_JWT_REFRESH_EXPIRATION_DAYS=7
    
    # Test Email Configuration
    TEST_SMTP_USERNAME=your_test_smtp_username
    TEST_SMTP_PASSWORD=your_test_smtp_password
    TEST_SMTP_SERVER=your_test_smtp_server
    TEST_FROM_EMAIL=test_noreply@yourdomain.com
    TEST_SUPPORT_EMAIL=test_support@yourdomain.com
    TEST_EMAIL_FROM_NAME="${TEST_APP_NAME} Support"
    TEST_EMAIL_VERIFICATION_SUBJECT="Verify Your ${TEST_APP_NAME} Account"
    TEST_EMAIL_PASSWORD_RESET_SUBJECT="Reset Your ${TEST_APP_NAME} Password"
    
    # Logging Configuration
    RUST_LOG=debug
    
    # Additional test configurations...
   ```
   Replace both files placeholders(.env and .env.test) with your actual database, SMTP, and other credentials.

3. Set up the database and build the application:

   ### 🍎 macOS Setup (Automated)
   
   For macOS users, we provide an automated setup script that handles everything:
   
   ```sh
   # Make the script executable (if not already)
   chmod +x setup-mac.sh
   
   # Run the automated setup script
   ./setup-mac.sh
   ```
   
   This script will automatically:
   - Install Homebrew (if not present)
   - Install Xcode Command Line Tools
   - Install Rust with WebAssembly support
   - Install Trunk for frontend builds
   - Install PostgreSQL 14 and start the service
   - Install SQLx CLI for database migrations
   - Create the database and run migrations
   - Install project dependencies
   - Test the complete setup
   - Create a `build-mac.sh` script for easy building

   After the setup completes, you can start the application with:
   ```sh
   ./build-mac.sh
   ```

   ### 🪟 Windows Setup
   
   For Windows users, run the PowerShell script to set up the database:
   ```sh
   .\setup-database.ps1
   ```
   This script will:
   - Check if PostgreSQL is installed
   - Create the database if it doesn't exist
   - Set up proper permissions for both superuser and application user
   - Run all necessary database migrations

   Then build and run the application:
   ```sh
   .\build.bat
   ```

   ### 🐧 Linux Setup
   
   For Linux users, you'll need to manually install the prerequisites and then:
   ```sh
   # Install dependencies (Ubuntu/Debian example)
   sudo apt-get update
   sudo apt-get install build-essential postgresql postgresql-contrib
   
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   
   # Add WebAssembly target
   rustup target add wasm32-unknown-unknown
   
   # Install Trunk and SQLx CLI
   cargo install trunk
   cargo install sqlx-cli --no-default-features --features native-tls,postgres
   
   # Set up database (adjust connection details as needed)
   sudo -u postgres createdb oxidizedoasis_db
   sqlx migrate run
   
   # Build and run
   cd frontend && trunk build && cd ..
   cargo run
   ```

   ### 📋 Manual Setup (All Platforms)
   
   If you prefer to set up manually or encounter issues with the automated scripts:
   
   ```sh
   # Build frontend
   cd frontend
   trunk build
   cd ..

   # Build and run backend
   cargo run
   ```

5. Visit `http://localhost:8080` in your browser to access the application.

## 🔧 Configuration

Refer to the [Software Development Document](docs/Software%20Development%20Document/TABLE%20OF%20CONTENTS.md) for detailed configuration instructions and environment variable descriptions.

## 📋 Development Workflow

The project follows a structured development workflow:

1. **Feature Planning**: New features are first documented in the appropriate documentation files
2. **Implementation**: Code is written following the project's architecture and coding standards
3. **Testing**: Comprehensive tests are written for all new features
4. **Review**: Code is reviewed for quality, security, and performance
5. **Deployment**: Changes are deployed using the CI/CD pipeline

## 🧪 Testing

We follow a test-driven development (TDD) approach. To run the tests:

```sh
cargo test
```

The project includes various types of tests:
- Unit tests for individual components
- Integration tests for API endpoints
- End-to-end tests for complete user flows
- Performance tests for critical paths

## 🔒 Security Features

Security is a top priority for OxidizedOasis-WebSands:

- **Authentication**: JWT-based with refresh tokens and revocation
- **Authorization**: Role-based access control for all resources
- **Data Protection**: All sensitive data is encrypted at rest and in transit
- **Input Validation**: Comprehensive validation for all user inputs
- **Output Encoding**: Proper encoding to prevent XSS attacks
- **CSRF Protection**: Token-based protection for all state-changing operations
- **Rate Limiting**: Protection against brute force and DoS attacks
- **Security Headers**: Properly configured security headers for all responses
- **Dependency Scanning**: Regular scanning for vulnerable dependencies

## 🚢 Deployment

The project can be deployed in several ways:

1. **Docker Deployment**:
   ```sh
   docker-compose up -d
   ```

2. **Manual Deployment**:
   - Build the frontend: `cd frontend && trunk build --release`
   - Build the backend: `cargo build --release`
   - Configure your web server to serve the static files and proxy API requests

3. **CI/CD Pipeline**:
   The project includes GitHub Actions workflows for automated testing and deployment.

Refer to the [Software Development Document](docs/Software%20Development%20Document/TABLE%20OF%20CONTENTS.md) for detailed deployment instructions.

## 📈 Roadmap

Future enhancements planned for OxidizedOasis-WebSands:

- Two-factor authentication (2FA) support
- OAuth2 integration for social login
- Enhanced admin dashboard with analytics
- Real-time notifications system
- API rate limiting and throttling
- Expanded user profile management
- Advanced permission system
- Audit logging for security events
- Performance optimizations for high-traffic scenarios

## 🤝 Contributing

Contributions are what make the open-source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

Please make sure to update tests as appropriate and adhere to the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).

## 📬 Contact

Daniel Biocchi

Project Link: [https://github.com/danbiocchi/oxidizedoasis-websands](https://github.com/danbiocchi/oxidizedoasis-websands)

## 🙏 Acknowledgements

- [Rust](https://www.rust-lang.org/)
- [Actix-web](https://actix.rs/)
- [Yew](https://yew.rs/)
- [SQLx](https://github.com/launchbadge/sqlx)
- [PostgreSQL](https://www.postgresql.org/)
- [JSON Web Tokens](https://jwt.io/)
- [bcrypt](https://en.wikipedia.org/wiki/Bcrypt)
- Diablo 2

---

<p align="center">
  Made in 🟥 🍁 🟥 with ❤️ by the OxidizedOasis-WebSands Team
</p>
