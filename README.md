Here is your updated, polished `README.md` for the Artisan CLI — with accurate commands, platform notes, and custom license reference:

---

````markdown
# 🛠️ Artisan CLI

Artisan CLI is a powerful developer tool to interact with the [Artisan Hosting](https://artisanhosting.net) platform. It provides secure, low-latency access to real-time status information, runner control, and node metrics — all from your terminal.

---

## 📦 Features

- 🧩 Node + Runner inspection
- 🔄 Control runners (start/stop/restart)
- 🔐 Secure token-based authentication
- 🔁 Automatic token refresh
- 🔍 Live output with `--watch` mode
- 📁 Portable `.env`-based configuration

---

## ⚙️ Installation

Clone and build the CLI using Cargo:

```bash
git clone https://github.com/yourusername/artisan_cli.git
cd artisan_cli
cargo build --release
````

The resulting binary will be located at:

```
./target/release/artisan_cli
```

This build process works across:

* ✅ Linux (x86\_64)
* ✅ macOS (Intel & Apple Silicon)
* ✅ Windows (x86\_64 GNU or MSVC)

---

## 🧪 Usage

Run the CLI like this:

```bash
artisan_cli <COMMAND> [OPTIONS]
```

### Available Commands

#### 🔐 Authentication

```bash
artisan_cli auth login <username> <password>
artisan_cli auth whoami
```

* Logs you in and stores a token securely in `~/.artisan_cli`
* Auto-refreshes tokens when possible

#### 📦 Instance Logs

```bash
artisan_cli logs <instance_id> <line_limit> [--watch N]
```

* Fetches logs from a running instance
* Add `--watch N` to auto-refresh every N seconds

#### 📊 Instance Status

```bash
artisan_cli status <instance_id> [--watch N]
```

* View memory, CPU, and bandwidth stats
* Refresh continuously with `--watch`

#### 🧠 Node & Runner Info

```bash
artisan_cli list-nodes
artisan_cli get-node <node_id>
artisan_cli list-runners
artisan_cli get-runner <runner_id>
```

#### 🧷 Runner Control

```bash
artisan_cli control <runner_id> <start|stop|restart>
```

---

## 🧩 Environment & Configuration

* First run creates: `~/.artisan_cli/.env`
* Tokens and encrypted credentials stored securely
* Use `dotenv` support for custom configs

---

## 👨‍💻 Development

```bash
# Run with default token + config
cargo run -- logs my-instance 100 --watch 5

# Build release binary
cargo build --release
```

---

## 📄 License

This project is licensed under the [AHSLv1](./License), a custom license developed for Artisan Hosting.

---

## 📬 Contact

Questions? Email: [dwhitfield@artisanhosting.net](mailto:dwhitfield@artisanhosting.net)

```
