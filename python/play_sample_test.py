import time
from pythonosc import udp_client

# Hardcoded default port of jdw-sc main application
client = udp_client.SimpleUDPClient("127.0.0.1", 13331) # Straight to main application

# NOTE: Since we have no built-in sample packs, this will only work if you have an example pack in /home
def play(index, category, args):
    client.send_message("/play_sample", [
        "test_sample_id", # External id for n_set reference
        "example", # Sample pack to use - a dir-name in "sample_packs"
        index, # Index in pack or category
        category # Category - blank equals none
    ] + args)

step = 0.4
short = step / 2

play(0, "bd", ["amp", 2.5])
time.sleep(step)
play(0, "sn", ["ofs", 0.13, "amp", 1.5])
time.sleep(step)
play(0, "bd", ["amp", 2.5])
time.sleep(short)
play(0, "bd", ["amp", 0.8, "ofs", 0.05])
time.sleep(short)
play(0, "to", ["amp", 2.5])
play(0, "sn", ["ofs", 0.13])
time.sleep(short)
play(0, "sn", ["ofs", 0.13])
time.sleep(short / 2)
play(0, "sn", ["ofs", 0.13])
time.sleep(short / 2)
play(0, "cy", ["ofs", 0.03, "sus", 2.0, "amp", 0.3])
