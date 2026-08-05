# Overmax Recommend Provider Protocol (v1)

This document describes the HTTP specification for external recommendation providers (e.g. community bots like Haeng-i, Rofe, djmax.gg) to supply recommended songs to Overmax overlay.

---

## Overview

Overmax acts as a **viewer** for recommended song entries. External services can expose two simple HTTP endpoints so that users can receive personalized/community recommendations directly in the in-game overlay without alt-tabbing.

Protocol Identifier: `overmax-recommend/1`

---

## 1. Endpoints

### 1.1 Manifest Endpoint (Optional but Recommended)

Provides provider metadata, cache TTL, and context dimensions to vary on.

```http
GET /manifest
```

#### Response (`200 OK`)

```json
{
  "protocol": "overmax-recommend/1",
  "name": "djmax.gg",
  "vary": ["mode"],
  "ttl_sec": 3600,
  "endpoint": "/recommend"
}
```

| Field | Type | Description |
|---|---|---|
| `protocol` | string | **Required**. Must be `"overmax-recommend/1"`. |
| `name` | string | Optional. Provider display name. |
| `vary` | string[] | Dimensions that trigger network cache invalidation. Subset of `["song_id", "mode", "diff", "v_id"]`. Empty array `[]` means fixed recommendations (e.g., daily recommendations). Default: `["song_id", "mode", "diff"]`. |
| `ttl_sec` | number | Cache Time-To-Live in seconds. Default: `3600`. |
| `endpoint` | string | Recommendation endpoint path (relative or absolute). Default: `"/recommend"`. |

---

### 1.2 Recommendation Endpoint (Required)

```http
GET {endpoint}?song_id={id}&mode={mode}&diff={diff}&v_id={v_id}
```

#### Query Parameters

| Parameter | Type | Example | Description |
|---|---|---|---|
| `song_id` | number | `123` | V-Archive Song ID. |
| `mode` | string | `5B` | Pattern mode: `"4B"`, `"5B"`, `"6B"`, `"8B"`. |
| `diff` | string | `SC` | Difficulty: `"NM"`, `"HD"`, `"MX"`, `"SC"`. |
| `v_id` | string | `user123` | User's V-Archive ID (empty string if unconfigured). |

#### Response (`200 OK`)

```json
{
  "protocol": "overmax-recommend/1",
  "source": "djmax.gg",
  "entries": [
    {
      "song_id": 123,
      "mode": "5B",
      "diff": "SC",
      "reason": "similar_tag",
      "score": 0.87
    }
  ]
}
```

| Field | Type | Description |
|---|---|---|
| `protocol` | string | **Required**. Must be `"overmax-recommend/1"`. |
| `source` | string | Provider identifier string. |
| `entries` | array | List of recommended song objects. |
| `entries[].song_id` | number | **Required**. V-Archive Song ID. |
| `entries[].mode` | string | **Required**. `"4B"`, `"5B"`, `"6B"`, `"8B"`. |
| `entries[].diff` | string | **Required**. `"NM"`, `"HD"`, `"MX"`, `"SC"`. |
| `entries[].reason` | string | Optional. Reason label (reserved for future UI expansion). |
| `entries[].score` | number | Optional. Score rating float (reserved for future UI expansion). |

---

## 2. Example Mock Server

An example Python mock server is available in `examples/recommend_mock_server.py`.
Run it locally:

```bash
python examples/recommend_mock_server.py
```
It will listen on `http://127.0.0.1:8080`.
