#!/bin/bash
echo "=== Kontrola instalace Tracker Agent ==="
echo ""
echo "1. Kde je nainstalovaná aplikace:"
which tracker-agent-app || echo "Příkaz 'tracker-agent-app' nenalezen v PATH"
echo ""
echo "2. Desktop soubor:"
find /usr/share/applications -name "*tracker*" 2>/dev/null
echo ""
echo "3. Binární soubor:"
find /usr -name "tracker-agent-app" -type f 2>/dev/null
echo ""
echo "4. Zkus spustit z terminálu:"
echo "tracker-agent-app"
