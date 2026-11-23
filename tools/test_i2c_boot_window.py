import urllib.request
import json
import time

BASE = "http://10.10.10.10"

def post(path, payload=None, retries=5):
    data = json.dumps(payload if payload is not None else {}).encode()
    req = urllib.request.Request(BASE + path, data=data,
                                 headers={"Content-Type": "application/json"},
                                 method="POST")
    for attempt in range(retries):
        try:
            with urllib.request.urlopen(req, timeout=5) as r:
                return json.loads(r.read().decode())
        except Exception as e:
            if attempt < retries - 1:
                time.sleep(0.05 * (attempt + 1))
                continue
            return {"err": str(e)}

def get(path):
    req = urllib.request.Request(BASE + path)
    with urllib.request.urlopen(req, timeout=5) as r:
        return json.loads(r.read().decode())

print("[*] Pulsing target nRST...")
post("/api/v1/programmer/reset", {})
time.sleep(0.02)

print("[*] Scanning I2C bus during boot window...")
scan = get("/api/v1/bus/i2c/scan")
print("I2C Scan:", scan)
