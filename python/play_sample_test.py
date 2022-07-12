import time
from pythonosc import udp_client

# Hardcoded default port of jdw-sc main application
client = udp_client.SimpleUDPClient("127.0.0.1", 13331) # Straight to main application

def play(index, category, args):
    client.send_message("/play_sample", [
        "example", # Sample pack to use - a dir-name in "sample_packs"
        index, # Index in pack or category
        category, # Category - blank equals none
        # Named args continue from here - see sampler.scd
    ] + args)

play(3, "", [])
time.sleep(0.2)
play(1, "", [])
time.sleep(0.2)
play(2, "", [])
time.sleep(0.2)
play(0, "", [])
play(1, "", [])
time.sleep(0.2)

play(0, "sn", [])
time.sleep(0.2)
play(0, "bd", [])
time.sleep(0.2)
play(0, "bd", [])
time.sleep(0.2)
play(0, "sn", [])
