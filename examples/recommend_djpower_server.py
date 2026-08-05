#!/usr/bin/env python3
"""
DJ POWER (official level) Recommendation Provider for Overmax.
Protocol: overmax-recommend/1

Recommends patterns that share the current pattern's official in-game level
(DJ POWER) rather than V-Archive's community floor rating. SC and NM/HD/MX
use independent 1-15 level scales that DJ POWER only treats as equivalent at
five points (SC1=NHM11, SC3=12, SC5=13, SC7=14, SC8=15); other SC levels have
no defined NHM equivalent and are kept isolated. This mirrors the level-based
recommendation logic previously explored in the fork's own Recommender
(rust/overmax_data/src/service/recommend.rs on feat/recommend-by-level),
reimplemented here as a standalone overmax-recommend/1 provider so it needs
no changes to Overmax itself.

Usage:
    python examples/recommend_djpower_server.py [--songs PATH] [--port PORT]

`--songs` defaults to cache/songs.json relative to the current directory
(Overmax's own V-Archive song DB - run this from the Overmax root, or pass
an explicit path). Point Overmax's Settings -> System -> 추천 Provider at
http://127.0.0.1:8090 (or whatever --port you choose) to consume it.
"""

import argparse
import json
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs

MODES = ["4B", "5B", "6B", "8B"]
DIFFICULTIES = ["NM", "HD", "MX", "SC"]
SC_TO_NHM_BRIDGE = {1: 11, 3: 12, 5: 13, 7: 14, 8: 15}

# The overmax-recommend/1 protocol has no floor_range/same_mode_only request
# params, so the provider owns these choices. Matches the fixed call-site
# defaults Overmax's own Recommender used (floor_range=0.0 exact match,
# same_mode_only=true, max_results=6).
MAX_RESULTS = 6


def dj_power_group(diff, level):
    """(group, value): NM/HD/MX always share the 1-15 "BRIDGED" scale. SC only
    joins it at the 5 known equivalence points; other SC levels are isolated
    into "SC_ONLY" since they have no defined NHM counterpart."""
    if diff == "SC":
        bridged = SC_TO_NHM_BRIDGE.get(level)
        if bridged is not None:
            return ("BRIDGED", bridged)
        return ("SC_ONLY", level)
    return ("BRIDGED", level)


def load_songs(path):
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def find_pattern(song, mode, diff):
    return song.get("patterns", {}).get(mode, {}).get(diff)


def recommend(songs, song_id, mode, diff):
    current = next((s for s in songs if s.get("title") == song_id), None)
    if current is None:
        return []
    pattern = find_pattern(current, mode, diff)
    if pattern is None or pattern.get("level") is None:
        return []

    ref_group, ref_value = dj_power_group(diff, pattern["level"])

    candidates = []
    for song in songs:
        sid = song.get("title")
        for d in DIFFICULTIES:
            p = find_pattern(song, mode, d)
            if p is None or p.get("level") is None:
                continue
            group, value = dj_power_group(d, p["level"])
            if group != ref_group or value != ref_value:
                continue
            if sid == song_id and d == diff:
                continue
            candidates.append(
                {
                    "song_id": sid,
                    "mode": mode,
                    "diff": d,
                    "reason": f"DJ POWER Lv.{value}",
                    "score": 1.0,
                }
            )
            if len(candidates) >= MAX_RESULTS:
                return candidates
    return candidates


class DjPowerHandler(BaseHTTPRequestHandler):
    songs = []

    def _send_json(self, payload):
        body = json.dumps(payload).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        parsed = urlparse(self.path)
        params = parse_qs(parsed.query)

        if parsed.path == "/manifest":
            self._send_json(
                {
                    "protocol": "overmax-recommend/1",
                    "name": "DJ POWER (Official Level)",
                    "vary": ["song_id", "mode", "diff"],
                    "ttl_sec": 300,
                    "endpoint": "/recommend",
                }
            )
        elif parsed.path == "/recommend":
            song_id = int(params.get("song_id", ["-1"])[0])
            mode = params.get("mode", ["4B"])[0]
            diff = params.get("diff", ["SC"])[0]

            print(
                f"[DjPowerServer] Received request: song_id={song_id}, mode={mode}, diff={diff}"
            )

            entries = recommend(self.songs, song_id, mode, diff)
            self._send_json(
                {
                    "protocol": "overmax-recommend/1",
                    "source": "DJ POWER (Official Level)",
                    "entries": entries,
                }
            )
        else:
            self.send_response(404)
            self.end_headers()


def run(songs_path, port):
    songs = load_songs(songs_path)
    DjPowerHandler.songs = songs
    server_address = ("127.0.0.1", port)
    httpd = HTTPServer(server_address, DjPowerHandler)
    print(
        f"DJ POWER recommend provider running on http://127.0.0.1:{port} "
        f"({len(songs)} songs loaded from {songs_path})"
    )
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nServer stopped.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--songs", default="cache/songs.json", help="Path to V-Archive songs.json"
    )
    parser.add_argument("--port", type=int, default=8090, help="Port to listen on")
    args = parser.parse_args()
    run(args.songs, args.port)
