# Service TLS test data

TLS tests generate temporary self-signed localhost certificates and an ephemeral RSA key using OpenSSL. No TLS private key is tracked or published. Temporary credentials are removed when the test completes; production trust stores are never modified.

Updater fixtures use two kinds of signed data: `updater-message.txt` is harmless text signed with the embedded app key to test byte/version binding, and is rejected as an app archive. `updater-valid.app.tar.gz` is a non-executable disposable test bundle signed by a separate test key (`updater-test.pub`); that key is never trusted by the production app. No private updater key is tracked. The integration test uses only an HTTP loopback server and a temporary mock app; production transport stays HTTPS-only.
