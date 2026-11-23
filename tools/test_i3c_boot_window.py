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

print("[*] Initializing I3C controller on AgentLA...")
r_on = post("/api/v1/bridge/i3c/on", {})
print("I3C On:", r_on)
time.sleep(0.05)
r_init = post("/api/v1/bus/i3c/init", {"freq_khz": 1000})
print("I3C Init:", r_init)

print("[*] Pulsing target nRST into BareBoot boot window...")
r_rst = post("/api/v1/programmer/reset", {})
print("Reset:", r_rst)

time.sleep(0.01)

print("[*] Running I3C ENTDAA (Dynamic Address Assignment)...")
r_daa = post("/api/v1/bus/i3c/entdaa", {"addr": 0x40})
print("ENTDAA:", r_daa)
