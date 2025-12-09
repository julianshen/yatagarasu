# Cache Warming API

The Cache Warming API allows administrators to pre-populate the cache layer.

**Base URL**: `http://<host>:8080/admin/cache/prewarm`

**Auth Required**: Yes (`Authorization: Bearer <ADMIN_TOKEN>`)

## Create Task

**POST** `/`

Start a new cache warming task.

**Body**:
```json
{
  "bucket": "my-bucket",
  "path": "folder/prefix/",
  "recursive": true
}
```

**Response**:
```json
{
  "status": "success",
  "task_id": "550e8400-e29b-...",
  "message": "Prewarm task created"
}
```

## List Tasks

**GET** `/tasks`

Returns all active and recent tasks.

**Response**:
```json
{
  "tasks": [
    {
      "id": "...",
      "bucket": "my-bucket",
      "status": "Running",
      "files_scanned": 150,
      "files_cached": 145
    }
  ]
}
```

## Get Task Status

**GET** `/status/{task_id}`

Get details for a specific task.

**Response**:
```json
{
  "id": "...",
  "status": "Completed",
  "start_time": "...",
  "end_time": "...",
  "files_cached": 1000
}
```

## Cancel Task

**DELETE** `/{task_id}`

Cancels a running task.
