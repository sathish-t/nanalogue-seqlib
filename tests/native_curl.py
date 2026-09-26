"""Local curl protocol regressions. Usage: python3 tests/native_curl.py CURL_TEST

CURL_TEST is native/curl-test.c linked to the direct target archives (see
native/check-curl-targets.rs). Python and openssl are fixture-only dependencies.
"""
import base64
import gzip
import http.server
import os
from pathlib import Path
import socket
import socketserver
import ssl
import subprocess
import sys
import tempfile
import threading


def main():
    client = str(Path(sys.argv[1]).resolve())
    root = Path(__file__).resolve().parent.parent
    payload = b"native transfer 0123456789: asymmetric payload"
    errors, seen = [], set()
    with tempfile.TemporaryDirectory(dir=root / "target", prefix="curl-protocols-") as directory:
        tmp = Path(directory)
        cert, key = tmp / "ca.pem", tmp / "key.pem"
        subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes",
                        "-keyout", str(key), "-out", str(cert), "-days", "1",
                        "-subj", "/CN=localhost", "-addext", "subjectAltName=DNS:localhost"],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(cert, key)

        class HTTP(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_GET(self):
                try:
                    if self.path == "/redirect":
                        self.send_response(302)
                        self.send_header("Location", "/compressed")
                        self.send_header("Set-Cookie", "fixture=kept; Path=/")
                        self.end_headers()
                        seen.add("redirect")
                        return
                    assert self.headers["Authorization"] == "Basic " + base64.b64encode(b"user:password").decode()
                    if self.path == "/compressed":
                        assert self.headers["Cookie"] == "fixture=kept"
                        assert "gzip" in self.headers["Accept-Encoding"]
                        body = gzip.compress(payload)
                        self.send_response(200)
                        self.send_header("Content-Encoding", "gzip")
                        seen.add("auth-cookie-gzip")
                    elif self.path == "/range":
                        assert self.headers["Range"] == "bytes=7-19"
                        body = payload[7:20]
                        self.send_response(206)
                        self.send_header("Content-Range", f"bytes 7-19/{len(payload)}")
                        seen.add("range")
                    else:
                        body = payload
                        self.send_response(200)
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)
                except Exception as error:
                    errors.append(repr(error))
                    self.send_error(500)

        class FTP(socketserver.BaseRequestHandler):
            def handle(self):
                connection = self.request
                listener = None
                secure_data = False
                try:
                    if self.server.implicit_tls:
                        connection = context.wrap_socket(connection, server_side=True)
                    def send(line):
                        connection.sendall((line + "\r\n").encode())
                    send("220 local fixture")
                    while True:
                        line = bytearray()
                        while not line.endswith(b"\r\n"):
                            char = connection.recv(1)
                            if not char:
                                return
                            line.extend(char)
                        command, _, argument = line.decode().strip().partition(" ")
                        if command == "AUTH":
                            send("234 proceed with TLS")
                            connection = context.wrap_socket(connection, server_side=True)
                        elif command == "USER":
                            assert argument == "user"
                            send("331 password required")
                        elif command == "PASS":
                            assert argument == "password"
                            send("230 logged in")
                        elif command == "PWD":
                            send('257 "/"')
                        elif command in ("TYPE", "PBSZ"):
                            send("200 OK")
                        elif command == "PROT":
                            secure_data = argument == "P"
                            send("200 OK")
                        elif command == "EPSV":
                            listener = socket.socket()
                            listener.bind(("127.0.0.1", 0))
                            listener.listen()
                            listener.settimeout(10)
                            send(f"229 Entering Extended Passive Mode (|||{listener.getsockname()[1]}|)")
                        elif command == "SIZE":
                            send(f"213 {len(payload)}")
                        elif command == "RETR":
                            assert argument == "fixture"
                            send("150 opening data connection")
                            data, _ = listener.accept()
                            if secure_data:
                                data = context.wrap_socket(data, server_side=True)
                            with data:
                                data.sendall(payload)
                            send("226 transfer complete")
                            seen.add("ftps" if secure_data else "ftp")
                        elif command == "QUIT":
                            send("221 goodbye")
                            return
                        else:
                            send("502 unsupported fixture command")
                except (ssl.SSLError, ConnectionResetError):
                    # A negative certificate test closes during the handshake.
                    pass
                except Exception as error:
                    errors.append(repr(error))
                finally:
                    if listener:
                        listener.close()
                    connection.close()

        plain = http.server.ThreadingHTTPServer(("127.0.0.1", 0), HTTP)
        https = http.server.ThreadingHTTPServer(("127.0.0.1", 0), HTTP)
        https.socket = context.wrap_socket(https.socket, server_side=True)
        ftp = socketserver.ThreadingTCPServer(("127.0.0.1", 0), FTP)
        ftps = socketserver.ThreadingTCPServer(("127.0.0.1", 0), FTP)
        ftp.implicit_tls, ftps.implicit_tls = False, True
        servers = [plain, https, ftp, ftps]
        try:
            class IPv6(http.server.ThreadingHTTPServer):
                address_family = socket.AF_INET6
            ipv6 = IPv6(("::1", 0), HTTP)
            servers.append(ipv6)
        except OSError:
            ipv6 = None
            print("IPv6 runtime unavailable on this host")
        threads = [threading.Thread(target=s.serve_forever, daemon=True) for s in servers]
        for thread in threads:
            thread.start()
        env = {k: v for k, v in os.environ.items()
               if k not in ("CURL_CA_BUNDLE", "SSL_CERT_FILE", "SSL_CERT_DIR")}

        def check(name, url, expected=payload, ca="-", byte_range="-", mode="-", code=0, **extra):
            result = subprocess.run([client, url, expected.decode(), str(ca), byte_range, mode],
                                    env=env | extra, capture_output=True, timeout=20)
            assert result.returncode == code, (name, result.returncode, result.stderr.decode(), errors)
            assert not errors, errors
            print(name + ": passed", flush=True)

        try:
            check("threaded localhost DNS, redirect, auth, cookies, gzip", f"http://localhost:{plain.server_port}/redirect")
            check("range", f"http://localhost:{plain.server_port}/range", payload[7:20], byte_range="7-19")
            if ipv6:
                check("IPv6 socket", f"http://[::1]:{ipv6.server_port}/fixture")
            tls_url = f"https://localhost:{https.server_port}/fixture"
            check("HTTPS untrusted rejected", tls_url, code=60)
            check("HTTPS hostname rejected", f"https://127.0.0.1:{https.server_port}/fixture", ca=cert, code=60)
            check("HTTPS per-request CA wins over bad environment", tls_url, ca=cert,
                  CURL_CA_BUNDLE=str(tmp / "absent"), SSL_CERT_FILE=str(tmp / "absent"))
            check("HTTPS bad per-request CA wins over valid environment", tls_url, ca=tmp / "absent",
                  SSL_CERT_FILE=str(cert), code=77)
            check("FTP", f"ftp://localhost:{ftp.server_address[1]}/fixture")
            check("explicit FTPS", f"ftp://localhost:{ftp.server_address[1]}/fixture", ca=cert, mode="tls")
            check("implicit FTPS untrusted rejected", f"ftps://localhost:{ftps.server_address[1]}/fixture", code=60)
            check("implicit FTPS", f"ftps://localhost:{ftps.server_address[1]}/fixture", ca=cert)
            assert {"redirect", "auth-cookie-gzip", "range", "ftp", "ftps"} <= seen, seen
        finally:
            for server in servers:
                server.shutdown()
                server.server_close()
            for thread in threads:
                thread.join()


if __name__ == "__main__":
    main()
