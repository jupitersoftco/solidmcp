#!/bin/sh -e
crwl crawl \
  -v \
  --deep-crawl bfs \
  --max-pages 1000 \
  -o markdown -O MCP-Specification-v2025-06-18.md \
  https://github.com/modelcontextprotocol/modelcontextprotocol/tree/main/docs/specification/2025-06-18

#  https://modelcontextprotocol.io/specification/2025-06-18
