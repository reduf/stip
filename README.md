# Stip

*Stip* is a TOTP token generator following [RFC6238](https://www.rfc-editor.org/rfc/rfc6238), that doesn't require a phone, that run natively on Windows, Linux and MacOs (maybe more?). In addition, it works without any remote service.

## How to compile

Download Rust following (or not) the steps described on [rust-lang.org](https://www.rust-lang.org/tools/install).

```
> git clone https://github.com/reduf/stip.git
> cd stip
> cargo build --release
```

The final executable can be found at `target/release/stip.exe`.

## How to use

*Stip* is currently a command line program that can output the TOTP token, reading a TOTP QR code, from the file system, from a decrypted zip file or from a encrypted zip file.

Here is an example of how to print the TOTP code from an image:

```
> stip.exe tests/data/noreply.example.png
```

In order to decrypt file from a zip archive, simply add the zip archive as a directory in the path, for instance:
```
> stip.exe tests/data/decrypted-noreply.example.zip/noreply.example.png
> stip.exe tests/data/encrypted-with-password-noreply.example.zip/noreply.example.png -p password
```

It may be desirable to not store your password in the command line argument, so you may prefer to use the flag `-p` or `--password` without an argument. When doing so, *stip* will prompt your for your password.

```
> stip.exe tests/data/encrypted-with-password-noreply.example.zip/noreply.example.png -p
Enter password:
```

## Icon attributions

- <a href="https://www.flaticon.com/free-icons/security" title="security icons">Security icons created by Freepik - Flaticon</a>
