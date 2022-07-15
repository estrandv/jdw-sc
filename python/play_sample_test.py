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
        "amp", 2.0
    ] + args)

step = 0.25

while True:

    play(1, "", ["amp", 2.5])
    time.sleep(step)
    play(1, "", ["ofs", 0.13])
    time.sleep(step)
    play(1, "", ["amp", 2.5])
    time.sleep(step)
    play(2, "", ["amp", 0.8, "ofs", 0.05])
    time.sleep(step)
