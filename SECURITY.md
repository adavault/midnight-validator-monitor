# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.9.x   | :white_check_mark: |
| < 0.9   | :x:                |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, use GitHub's private vulnerability reporting feature:

1. Go to the **Security** tab of this repository
2. Click **Report a vulnerability**
3. Fill out the form with details about the vulnerability

You can also navigate directly to: `https://github.com/adavault/midnight-validator-monitor/security/advisories/new`

### What to Include

- Type of vulnerability (e.g., SQL injection, path traversal, information disclosure)
- Location of the affected code (file path and line numbers if known)
- Step-by-step instructions to reproduce
- Proof-of-concept or exploit code (if available)
- Impact assessment

### Response Timeline

- **Initial response:** Within 48 hours
- **Status update:** Within 7 days
- **Fix timeline:** Depends on severity (critical: ASAP, high: 30 days, medium/low: next release)

### After Reporting

1. You'll receive an acknowledgment within 48 hours
2. We'll investigate and keep you updated on progress
3. Once fixed, we'll coordinate disclosure timing with you
4. Security fixes will be released with a new version and CVE (if applicable)

## Security Best Practices for Users

### Keystore Security

- Keep your keystore directory permissions restricted: `chmod 700 /path/to/keystore`
- Never share keystore files or their contents
- Use separate keystores for testnet and mainnet

### Configuration Security

- If your config file contains keystore paths, restrict permissions: `chmod 600 ~/.config/mvm/config.toml`
- Prefer using the systemd service over environment variables for sensitive paths
- Use HTTPS for RPC endpoints when connecting to remote nodes

### Network Security

- Run your validator node's RPC on localhost only, or use a reverse proxy with authentication
- The `--rpc-methods=unsafe` flag (needed for key verification) should only be used on trusted networks

## Scope

This security policy covers the MVM (Midnight Validator Monitor) CLI tool. It does **not** cover:

- The Midnight blockchain node software
- Third-party dependencies (report those to their respective maintainers)
- Infrastructure or deployment issues specific to your environment

## Recognition

We appreciate security researchers who help keep MVM secure. Contributors who report valid vulnerabilities will be acknowledged in release notes (unless they prefer to remain anonymous).
