# Artisan CLI

Artisan CLI is a command-line tool designed to interact with the Artisan platform, allowing users to manage nodes, runners, and authentication seamlessly. It supports fetching details about nodes and runners, sending commands, and authenticating with token-based security.

---

## Features
- **Node Management**: List nodes, fetch node details.
- **Runner Management**: List runners, fetch runner details, and control runners.
- **Authentication**: Login with credentials and refresh tokens automatically.
- **Customizable Environment**: Specify custom `.env` file paths for configuration.

---

## Prerequisites
- **Rust**: Ensure you have Rust installed. If not, follow the [installation guide](https://www.rust-lang.org/tools/install).
- **Cargo**: Comes with Rust installation.

---

## Installation
Clone the repository and build the project:

```sh
# Clone the repository
$ git clone https://github.com/yourusername/artisan_cli.git

# Navigate into the project directory
$ cd artisan_cli

# Build the project
$ cargo build --release
```

---

## Configuration
The application automatically creates and manages a `.env` file at `{home}/.artisan_cli/.env` to store environment variables such as `API_TOKEN`. This ensures that configurations persist between sessions without manual setup.

---

## Usage
Run the CLI with the following commands:

```sh
# Authenticate and acquire a new token
$ artisan_cli login yourusername yourpassword

# List all nodes
$ artisan_cli list-nodes

# Get details of a specific node
$ artisan_cli get-node --node-id node123

# List all runners
$ artisan_cli list-runners

# Get details of a specific runner
$ artisan_cli get-runner-details --runner-id runner123

# Control a runner (e.g., start, stop, restart)
$ artisan_cli control-runner --runner-id runner123 --command start
```

---

## Token Management
- **Automatic Refresh**: Tokens are refreshed automatically if they are close to expiration.
- **Encrypted Credentials**: Login credentials are stored in an encrypted file managed by the application at `~/.artisan_cli/credentials.ejson`.

The application ensures secure storage and access to these credentials, preventing unauthorized access.

## Development
### Running in Debug Mode
```sh
$ cargo run
```

---

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

---

## License
[AHSLv1](License)

---

## Contact
For any questions or support, feel free to reach out at yourusername@artisanhosting.com.

