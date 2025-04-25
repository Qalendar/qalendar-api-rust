# Qalendar API

<!-- [![Build Status](URL_TO_YOUR_CI_BADGE)](URL_TO_YOUR_CI_PIPELINE) -->
<!-- Optional: Add CI Badge -->
<!-- [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) -->
<!-- Optional: Add License Badge -->

The backend API server for the Qalendar application, built with Rust, Axum, SQLx, and PostgreSQL.

## Overview

This API provides endpoints for:

* User Authentication (Registration, Login, Email Verification, Password Reset)
* Managing user-owned data:
  * Categories
  * Deadlines
  * Events (including recurring events via RRULE)
  * Event Invitations (sending and responding)
  * Calendar Sharing (creating and managing shares)
* Viewing consolidated calendar data (owned items + accepted invites)
* Viewing shared calendar data (respecting privacy levels and category filters)
* Data synchronization for clients (fetching updates since a specific time)

## Getting Started

There are two main ways to get the API server running: building from source or using pre-built artifacts. Both methods require a running PostgreSQL database instance.

### Prerequisites

1. **PostgreSQL Database:** Ensure you have a PostgreSQL server running (latest recommended). Create a database and user for the API. You'll need the connection string later.
2. **Git:** Required to clone the repository if building from source.
3. **(Optional but Recommended for Email Features):** An SMTP service provider account (e.g., SendGrid, Mailgun, AWS SES, Gmail with App Password) to get SMTP credentials for sending verification and reset emails.

### Option A: Building and Running from Source (Recommended for Development)

This method requires the Rust toolchain.

1. **Install Rust:** If you don't have Rust installed, follow the instructions at [rustup.rs](https://rustup.rs/).
2. **Clone Repository:**

    ```zsh
    git clone https://github.com/Qalendar/qalendar-api-rust
    cd Qalendar/qalendar-api-rust
    ```

3. **Install `sqlx-cli` (Recommended for Migrations):**

    ```zsh
    cargo install sqlx-cli
    ```

4. **Set up Database & Apply Schema:**
    * Ensure your PostgreSQL server is running.
    * Create a database for Qalendar (e.g., `qalendar_db`) and a user with privileges for this database. You can do this using a client tool or `sqlx-cli` if you have it installed:

        ```zsh
        # Using sqlx-cli (requires DATABASE_URL env var set temporarily)
        export DATABASE_URL=postgres://username:password@localhost:5432/qalendar_db # Set this first
        sqlx database create
        ```

    * Apply the database schema by executing the SQL commands in the [`sql/setup.sql`](sql/setup.sql) file. You can do this using a PostgreSQL client tool (like `psql`):

        ```zsh
        psql -d qalendar_db -U username -f sql/setup.sql
        ```

        Replace `qalendar_db` and `username` with your database and user details.

5. **Create Configuration File:** Copy the `.env.example` file to `.env` in the `qalendar-api` directory and fill in your actual database credentials, JWT secret, SMTP details, and other configuration (see [Configuration](#configuration) below).

    ```zsh
    cp .env.example .env
    # Now edit .env with your details
    ```

6. **Build and Run:**
    * **Development:**

        ```zsh
        cargo run
        ```

    * **Release (Optimized):**

        ```zsh
        cargo build --release
        # The executable will be in ./target/release/qalendar-api
        # Make sure the .env file is in the same directory you run the executable from,
        # or configure environment variables directly in your shell.
        ./target/release/qalendar-api
        ```

### Option B: Using Pre-built Artifacts

You can download pre-compiled binaries for your operating system directly from the GitHub repository's Releases page or the build artifacts from GitHub Actions workflows (if configured).

1. **Download:** Go to the [GitHub Actions](https://github.com/Qalendar/qalendar-api-rust/actions) page and download the executable file (only available for Windows 64-bit for now). You might need to unzip the downloaded file.
2. **Database Setup:** Ensure your PostgreSQL database is running. Create a database and a user with privileges. Apply the database schema by executing the SQL commands found in the [`sql/setup.sql`](sql/setup.sql) file using a PostgreSQL client tool (like `psql`, PGAdmin, DBeaver).
3. **Create Configuration File:** Create a file named `.env` in the *same directory* where you placed the downloaded `qalendar-api` executable. Copy the contents from `example.env` (available in the source repository) and fill in your configuration details (see [Configuration (.env File)](#configuration-env-file) below).
4. **Run:** Open a terminal or command prompt in the directory containing the executable and the `.env` file, and run the server:

    <!-- ```zsh
    # Linux/macOS (ensure executable permission: chmod +x qalendar-server-...)
    ./qalendar-server-<your-platform>
    ``` -->

    * Windows

        ```pwsh
        .\qalendar-api.exe
        ```

---

## Configuration (`.env` File)

The API server requires configuration via environment variables. The easiest way is to create a `.env` file in the `qalendar-api` directory (when building from source) or alongside the executable (when using pre-built artifacts).

Please refer to the `.env.example` file in the repository for the most up-to-date list of variables. Key variables include:

* **`DATABASE_URL`**: Your PostgreSQL connection string (e.g., `postgres://user:password@host:port/database`).
* **`JWT_SECRET`**: A strong, unique secret string for signing authentication tokens. **Keep this secure!**
* **`SERVER_ADDRESS`**: The address and port the server should listen on (e.g., `0.0.0.0:8000`).
* **`JWT_EXPIRATION_SECONDS`**: How long authentication tokens are valid (e.g., `86400` for 1 day).
* **`SMTP_SERVER`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASSWORD`, `SENDER_EMAIL`, `SENDER_NAME`**: Credentials for your email service provider (needed for email verification and password reset). Ensure `SENDER_EMAIL` is authorized by your provider.
* **`VERIFICATION_CODE_EXPIRES_MINUTES`**: How long email verification codes are valid.
* **`RESET_CODE_EXPIRES_MINUTES`**: How long password reset codes are valid.
* **`FRONTEND_URL`**: The base URL of your Qalendar frontend application (e.g., `http://localhost:3000`, `https://qalendar.app`). This is used to construct links in emails.

**Security Note:** Do **NOT** commit your actual `.env` file containing secrets to version control. Ensure it is listed in your project's `.gitignore` file.

---

## Running the API

Once configured (and built, if using source), run the server using the commands mentioned in the setup sections (`cargo run` or `./qalendar-api`).

The server will log output to the console, indicating the address it's listening on (e.g., `Server listening on 0.0.0.0:8000`). You can interact with the API using tools like `curl`, Postman, Insomnia, or directly from your frontend application.

---

## API Documentation

Detailed documentation for all available API endpoints, including request/response formats and examples, can be found in:

**[`DOCUMENTATION.md`](./DOCUMENTATION.md)**

**Endpoint Categories:**

* `/api/auth`: User authentication and account management.
* `/api/me`: Managing data owned by the authenticated user (categories, deadlines, events, invitations, shares). Requires authentication.
* `/api/calendar`: Viewing consolidated calendar data (own and shared). Requires authentication.
* `/api/sync`: Endpoints for synchronizing data with the client. Requires authentication.

<!-- ---

## Contributing

*(Optional: Add guidelines for contributing if this is an open project)*

We welcome contributions! Please see `CONTRIBUTING.md` (if you create one) for details on how to contribute, report issues, and submit pull requests.

---

## License

*(Optional: Specify the project license)*

This project is licensed under the MIT License - see the `LICENSE` file for details. -->
