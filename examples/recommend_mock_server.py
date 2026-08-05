#!/usr/bin/env python3
"""
Example Mock Recommendation Provider Server for Overmax.
Protocol: overmax-recommend/1
"""

from http.server import HTTPServer, BaseHTTPRequestHandler
import json
from urllib.parse import urlparse, parse_qs

class MockRecommendHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        parsed = urlparse(self.path)
        path = parsed.path
        params = parse_qs(parsed.query)

        if path == "/manifest":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            response = {
                "protocol": "overmax-recommend/1",
                "name": "Mock Provider",
                "vary": ["mode"],
                "ttl_sec": 300,
                "endpoint": "/recommend"
            }
            self.wfile.write(json.dumps(response).encode("utf-8"))

        elif path == "/recommend":
            song_id = params.get("song_id", ["0"])[0]
            mode = params.get("mode", ["4B"])[0]
            diff = params.get("diff", ["SC"])[0]
            v_id = params.get("v_id", [""])[0]

            print(f"[MockServer] Received request: song_id={song_id}, mode={mode}, diff={diff}, v_id={v_id}")

            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()

            # Dummy recommendations (using valid V-Archive song IDs 14: Far East Princess, 16: glory day)
            response = {
                "protocol": "overmax-recommend/1",
                "source": "Mock Provider",
                "entries": [
                    {
                        "song_id": 14,
                        "mode": mode,
                        "diff": "SC",
                        "reason": "Popular Choice",
                        "score": 0.95
                    },
                    {
                        "song_id": 16,
                        "mode": mode,
                        "diff": "MX",
                        "reason": "Personalized Recommendation",
                        "score": 0.88
                    }
                ]
            }
            self.wfile.write(json.dumps(response).encode("utf-8"))
        else:
            self.send_response(404)
            self.end_headers()

def run(port=8080):
    server_address = ("127.0.0.1", port)
    httpd = HTTPServer(server_address, MockRecommendHandler)
    print(f"Mock Recommendation Server running on http://127.0.0.1:{port}")
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nServer stopped.")

if __name__ == "__main__":
    run()
