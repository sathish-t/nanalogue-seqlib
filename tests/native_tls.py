"""Local TLS/CA/proxy/S3/GCS regressions; no cloud access or real credentials.

Usage: python3 tests/native_tls.py OUT_DIR/native [x86_64-linux-gnu]
Requires Python, system openssl (certificate fixtures only), and Zig 0.15.2.
The prefix must come from a Cargo --all-features build. Scratch stays in target.
"""
import hashlib
import hmac
import http.server
import os
from pathlib import Path
import select
import socket
import ssl
import subprocess
import sys
import tempfile
import threading
from urllib.parse import urlsplit


def main():
    root = Path(__file__).resolve().parent.parent
    prefix = Path(sys.argv[1]).resolve()
    target = sys.argv[2] if len(sys.argv) > 2 else "x86_64-linux-gnu"
    with tempfile.TemporaryDirectory(prefix="native-tls-", dir=root / "target") as tmp:
        tmp = Path(tmp)
        cert, key = tmp / "ca.pem", tmp / "key.pem"
        subprocess.run([
            "openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes",
            "-keyout", str(key), "-out", str(cert), "-days", "1",
            "-subj", "/CN=local-test", "-addext",
            "subjectAltName=IP:127.0.0.1,DNS:localhost,DNS:test.storage-download.googleapis.com",
        ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        client = tmp / "client"
        subprocess.run([
            os.environ.get("ZIG", "zig"), "cc", "-target", target, "-mcpu=baseline",
            "-O2", "-fsanitize=undefined", "-fsanitize-trap=undefined",
            "-I" + str(prefix / "include"), str(root / "tests/native_tls.c"),
            *[str(prefix / "lib" / ("lib" + lib + ".a"))
              for lib in ("hts", "curl", "ssl", "crypto", "deflate", "lzma", "bz2", "z")],
            "-pthread", "-ldl", "-lm", "-o", str(client),
        ], check=True)
        bam = (root / "test/test.bam").read_bytes()
        seen, errors, connects = [], [], []

        class Origin(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_GET(self):
                try:
                    if self.path.startswith("/bucket/"):
                        # Independently verify SigV4, not merely the presence of a header.
                        auth = self.headers["Authorization"]
                        assert auth.startswith("AWS4-HMAC-SHA256 ")
                        fields = dict(x.strip().split("=", 1) for x in auth.split(" ", 1)[1].split(","))
                        access, scope = fields["Credential"].split("/", 1)
                        assert access == "TESTKEY"
                        signed = fields["SignedHeaders"]
                        headers = "".join(f"{h}:{self.headers[h]}\n" for h in signed.split(";"))
                        url = urlsplit(self.path)
                        canonical = "\n".join(["GET", url.path, url.query, headers, signed,
                                               self.headers["x-amz-content-sha256"]])
                        string = "\n".join(["AWS4-HMAC-SHA256", self.headers["x-amz-date"],
                                            scope, hashlib.sha256(canonical.encode()).hexdigest()])
                        signing = b"AWS4test-secret"
                        for part in scope.split("/"):
                            signing = hmac.new(signing, part.encode(), hashlib.sha256).digest()
                        assert hmac.new(signing, string.encode(), hashlib.sha256).hexdigest() == fields["Signature"]
                        seen.append("s3")
                    elif self.headers.get("Host", "").startswith("test.storage-download"):
                        assert self.headers["Authorization"] == "Bearer test-token"
                        assert self.headers["X-Goog-User-Project"] == "test-project"
                        seen.append("gcs")
                    self.send_response(200)
                    self.send_header("Content-Length", str(len(bam)))
                    self.end_headers()
                    self.wfile.write(bam)
                except Exception as error:
                    errors.append(repr(error))
                    self.send_error(500)

        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(cert, key)
        origin = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Origin)
        origin.socket = context.wrap_socket(origin.socket, server_side=True)

        class Proxy(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_CONNECT(self):
                connects.append(self.path)
                # Never contact the requested host: all traffic is a local fixture.
                with socket.create_connection(origin.server_address) as upstream:
                    self.send_response(200)
                    self.end_headers()
                    self.wfile.flush()
                    self.close_connection = True
                    while True:
                        ready, _, _ = select.select([self.connection, upstream], [], [], 10)
                        if isinstance(self.connection, ssl.SSLSocket) and self.connection.pending():
                            ready.append(self.connection)
                        if not ready:
                            return
                        for source in set(ready):
                            data = source.recv(65536)
                            if not data:
                                return
                            destination = upstream if source is self.connection else self.connection
                            destination.sendall(data)

        proxy = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Proxy)
        proxy.socket = context.wrap_socket(proxy.socket, server_side=True)
        servers = [origin, proxy]
        threads = [threading.Thread(target=s.serve_forever, daemon=True) for s in servers]
        for thread in threads:
            thread.start()
        env = {k: v for k, v in os.environ.items()
               if not (k.lower().endswith("_proxy") or k.startswith(("AWS_", "HTS_S3", "GCS_"))
                       or k in ("CURL_CA_BUNDLE", "SSL_CERT_FILE", "SSL_CERT_DIR"))}
        env.update(HOME=str(tmp), NO_PROXY="*", AWS_SHARED_CREDENTIALS_FILE=str(tmp / "absent"))
        url = f"https://127.0.0.1:{origin.server_port}/test.bam"

        def check(name, address=url, ok=True, **settings):
            result = subprocess.run([str(client), address, str(cert)], env=env | settings,
                                    capture_output=True, timeout=30)
            if result.returncode < 0 and os.environ.get("NATIVE_TLS_GDB"):
                subprocess.run(["gdb", "-batch", "-ex", "run", "-ex", "bt 12",
                                "--args", str(client), address, str(cert)], env=env | settings, timeout=30)
            assert (result.returncode == 0) == ok, (name, result.returncode, result.stderr.decode(), errors)
            # A crash must never count as an expected certificate rejection.
            if not ok:
                assert result.returncode == 1, (name, result.returncode)
            print(name + ": passed", flush=True)

        try:
            check("untrusted certificate rejected", ok=False)
            check("CURL_CA_BUNDLE", CURL_CA_BUNDLE=str(cert))
            check("SSL_CERT_FILE", SSL_CERT_FILE=str(cert))
            check("explicit bad bundle wins", ok=False, CURL_CA_BUNDLE=str(tmp / "absent"), SSL_CERT_FILE=str(cert))
            check("explicit empty bundle respected", ok=False, CURL_CA_BUNDLE="", SSL_CERT_FILE=str(cert))
            ca_dir = tmp / "certs"
            ca_dir.mkdir()
            (ca_dir / "ca.pem").write_bytes(cert.read_bytes())
            subprocess.run(["openssl", "rehash", str(ca_dir)], check=True)
            check("SSL_CERT_DIR", SSL_CERT_DIR=str(ca_dir))
            proxy_env = dict(https_proxy=f"https://127.0.0.1:{proxy.server_port}",
                             NO_PROXY="", SSL_CERT_FILE=str(cert))
            check("HTTPS proxy plus HTTPS origin", **proxy_env)
            check("S3 SigV4 over TLS", address="s3://bucket/test.bam", SSL_CERT_FILE=str(cert),
                  HTS_S3_HOST=f"127.0.0.1:{origin.server_port}", HTS_S3_ADDRESS_STYLE="path",
                  AWS_ACCESS_KEY_ID="TESTKEY", AWS_SECRET_ACCESS_KEY="test-secret",
                  AWS_DEFAULT_REGION="us-east-1")
            check("GCS bearer/requester-pays over TLS", address="gs://test/test.bam",
                  GCS_OAUTH_TOKEN="test-token", GCS_REQUESTER_PAYS_PROJECT="test-project", **proxy_env)
            assert "s3" in seen and "gcs" in seen and len(connects) >= 2, (seen, connects)
            assert not errors, errors
        finally:
            for server in servers:
                server.shutdown()
                server.server_close()
            for thread in threads:
                thread.join()


if __name__ == "__main__":
    main()
