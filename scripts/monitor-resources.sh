#!/bin/bash
# Resource monitoring script for Yatagarasu
# Usage: ./scripts/monitor-resources.sh > metrics.log

echo "=== Resource Monitoring Started at $(date) ==="
echo "Monitoring Yatagarasu proxy container..."
echo ""

while true; do
  # Get timestamp
  TIMESTAMP=$(date +%s)
  DATETIME=$(date '+%Y-%m-%d %H:%M:%S')

  # Check if container is running
  if ! docker ps --format '{{.Names}}' | grep -q "yatagarasu-proxy"; then
    echo "[$DATETIME] ERROR: Yatagarasu container not found"
    sleep 10
    continue
  fi

  # Get container stats (one-shot, no stream)
  STATS=$(docker stats yatagarasu-proxy --no-stream --format "{{.CPUPerc}},{{.MemUsage}},{{.NetIO}},{{.BlockIO}},{{.PIDs}}")

  # Parse stats
  CPU=$(echo $STATS | cut -d',' -f1)
  MEM=$(echo $STATS | cut -d',' -f2 | awk '{print $1}')  # Just the used memory
  NET=$(echo $STATS | cut -d',' -f3)
  BLOCK=$(echo $STATS | cut -d',' -f4)
  PIDS=$(echo $STATS | cut -d',' -f5)

  # Print metrics
  echo "[$DATETIME] CPU=$CPU MEM=$MEM NET=$NET BLOCK=$BLOCK PIDS=$PIDS"

  sleep 10
done
