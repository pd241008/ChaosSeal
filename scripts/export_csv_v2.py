import json, glob, csv, sys
from analysis.stats import goodput_mbps, throughput_mbps

runs = []
for p in sorted(glob.glob("results_v2/*.json")):
    with open(p) as f: runs.append(json.load(f))

# Gather unique R values
rs = set()
for r in runs:
    rv = r.get("parameters", {}).get("bee_r")
    if rv is not None:
        rs.add(rv)

rs = sorted(list(rs))

with open("raw_output_v2.csv", "w", newline="") as f:
    writer = csv.writer(f)
    writer.writerow(["Scenario", "Protocol", "PayloadBytes", "OverheadBytes", "Throughput_Mbps", "Goodput_Mbps", "CryptoWallclock_us"])
    
    for rv in rs:
        # Find all runs with this R
        r_runs = [r for r in runs if r.get("parameters", {}).get("bee_r") == rv and "bpsec" in r.get("baselines", {})]
        
        for r in r_runs:
            bs = r["baselines"]
            
            # BPSec
            bp = bs.get("bpsec")
            if bp:
                sec = bp["transfer_sec"]
                writer.writerow([f"R={rv}", "BPSec", bp["payload_bytes"], bp["ciphertext_bytes"], 
                                 (bp["bundle_size_bytes"]*8/sec/1e6), (bp["payload_bytes"]*8/sec/1e6), "N/A"])
            
            # ChaosSeal
            cs = bs.get("chaosseal", {}).get("data_transmission")
            if cs:
                sec = cs["transfer_sec"] + (cs.get("crypto_wallclock_us", 0)/1e6)
                writer.writerow([f"R={rv}", "ChaosSeal", cs["payload_bytes"], cs["payload_bytes"] * cs["overhead_pct"],
                                 (cs["payload_bytes"]*(1+cs["overhead_pct"])*8/sec/1e6), (cs["payload_bytes"]*8/sec/1e6), cs.get("crypto_wallclock_us", 0)])

print("Wrote raw_output_v2.csv")
